#![no_std]
#![no_main]

use core::str::from_utf8;

#[cfg(not(any(target_os = "stax", target_os = "flex")))]
use display::MainPages;
#[cfg(any(target_os = "stax", target_os = "flex"))]
use ledger_device_sdk::nbgl::{NbglHomeAndSettings, init_comm};
use error_code::ErrorCode;
use public_key::derive_pub_key;
#[cfg(any(target_os = "stax", target_os = "flex"))]
use settings::SETTINGS_DATA;
use sign_tx_context::SignTxContext;
use tx_reviewer::TxReviewer;
use utils::{self, deserialize_path};

#[cfg(not(any(target_os = "stax", target_os = "flex")))]
mod display;
#[cfg(any(target_os = "stax", target_os = "flex"))]
mod nbgl;
mod blake2b_hasher;
mod settings;
mod debug;
mod error_code;
mod public_key;
mod sign_tx_context;
mod tx_reviewer;
mod ledger_sdk_stub;

use debug::print::{println, println_slice};
use ledger_device_sdk::{ecc::{Secp256k1, SeedDerive}, io};

ledger_device_sdk::set_panic!(ledger_device_sdk::exiting_panic);

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();

    let mut sign_tx_context: SignTxContext = SignTxContext::new();
    let mut tx_reviewer: TxReviewer = TxReviewer::new();

    #[cfg(not(any(target_os = "stax", target_os = "flex")))]
    {
        let mut main_pages = MainPages::new();
        loop {
            // Wait for either a specific button push to exit the app
            // or an APDU command
            match main_pages.show(&mut comm) {
                io::Event::Command(ins) => {
                    match handle_apdu(&mut comm, ins, &mut sign_tx_context, &mut tx_reviewer) {
                        Ok(()) => comm.reply_ok(),
                        Err(sw) => comm.reply(sw),
                    }
                    main_pages.show_ui(main_pages.ui_index);
                }
                _ => ()
            }
        }
    }

    #[cfg(any(target_os = "stax", target_os = "flex"))]
    {
        init_comm(&mut comm);
        let settings_strings = [["Blind signing", "Enable blind signing"]];
        loop {
            let event = NbglHomeAndSettings::new()
                .settings(unsafe { SETTINGS_DATA.get_mut() }, &settings_strings)
                .infos("Alephium", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_AUTHORS"))
                .show::<Ins>();
            match event {
                io::Event::Command(ins) => {
                    match handle_apdu(&mut comm, ins, &mut sign_tx_context, &mut tx_reviewer) {
                        Ok(_) => comm.reply_ok(),
                        Err(sw) => comm.reply(sw)
                    }
                }
                _ => ()
            }
        }
    }
}

#[repr(u8)]
pub enum Ins {
    GetVersion,
    GetPubKey,
    SignHash,
    SignTx,
}

impl TryFrom<io::ApduHeader> for Ins {
    type Error = ErrorCode;
    fn try_from(header: io::ApduHeader) -> Result<Self, Self::Error> {
        match header.ins {
            0 => Ok(Ins::GetVersion),
            1 => Ok(Ins::GetPubKey),
            2 => Ok(Ins::SignHash),
            3 => Ok(Ins::SignTx),
            _ => Err(ErrorCode::BadIns),
        }
    }
}

fn handle_apdu(
    comm: &mut io::Comm,
    ins: Ins,
    sign_tx_context: &mut SignTxContext,
    tx_reviewer: &mut TxReviewer,
) -> Result<(), io::Reply> {
    if comm.rx == 0 {
        return Err(ErrorCode::BadLen.into());
    }

    let mut path: [u32; 5] = [0; 5];
    let apdu_header = comm.get_apdu_metadata();
    if apdu_header.cla != 0x80 {
        return Err(ErrorCode::BadCla.into());
    }

    match ins {
        Ins::GetVersion => {
            let version_major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u8>().unwrap();
            let version_minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u8>().unwrap();
            let version_patch = env!("CARGO_PKG_VERSION_PATCH").parse::<u8>().unwrap();
            comm.append([version_major, version_minor, version_patch].as_slice());
        }
        Ins::GetPubKey => {
            let raw_path = comm.get_data()?;
            println("raw path");
            println_slice::<40>(raw_path);
            if !deserialize_path(raw_path, &mut path) {
                return Err(ErrorCode::BadLen.into());
            }

            let p1 = apdu_header.p1;
            let p2 = apdu_header.p2;
            let (pk, hd_index) = derive_pub_key(&mut path, p1, p2)?;

            comm.append(pk.as_ref());
            comm.append(hd_index.to_be_bytes().as_slice());
        }
        Ins::SignHash => {
            let data = comm.get_data()?;
            if data.len() != 4 * 5 + 32 {
                return Err(ErrorCode::BadLen.into());
            }
            // This check can be removed, but we keep it for double checking
            if !deserialize_path(&data[..20], &mut path) {
                return Err(ErrorCode::BadLen.into());
            }

            match sign_ui(&path, &data[20..]) {
                Ok((signature_buf, length, _)) => comm.append(&signature_buf[..length as usize]),
                Err(code) => return Err(code.into()),
            }
        }
        Ins::SignTx => {
            let data = comm.get_data()?;
            match sign_tx_context.handle_data(apdu_header, data, tx_reviewer) {
                Ok(()) if !sign_tx_context.is_complete() => {
                    return Ok(());
                }
                Ok(()) => {
                    let result = match sign_tx_context.review_tx_id_and_sign() {
                        Ok((signature_buf, length, _)) => {
                            comm.append(&signature_buf[..length as usize]);
                            Ok(())
                        }
                        Err(code) => Err(code.into()),
                    };
                    sign_tx_context.reset();
                    return result;
                }
                Err(code) => {
                    sign_tx_context.reset();
                    return Err(code.into());
                }
            }
        }
    }
    Ok(())
}

fn sign_ui(path: &[u32], message: &[u8]) -> Result<([u8; 72], u32, u32), ErrorCode> {
    let hex: [u8; 64] = utils::to_hex(message).map_err(|_| ErrorCode::BadLen)?;
    let hex_str = from_utf8(&hex).map_err(|_| ErrorCode::InternalError)?;

    #[cfg(not(any(target_os = "stax", target_os = "flex")))]
    {
        use crate::ledger_sdk_stub::multi_field_review::MultiFieldReview;
        use ledger_device_sdk::ui::{bitmaps::{CHECKMARK, CROSS, EYE}, gadgets::Field};

        let review_messages = ["Review ", "Hash "];
        let fields = [Field{ name: "Hash", value: hex_str }];
        let review = MultiFieldReview::new(
            &fields,
           &review_messages,
           Some(&EYE),
           "Approve",
           Some(&CHECKMARK),
           "Reject",
           Some(&CROSS),
        );
        if review.show() {
            sign_hash(path, message)
        } else {
            Err(ErrorCode::UserCancelled)
        }
    }

    #[cfg(any(target_os = "stax", target_os = "flex"))]
    {
        use ledger_device_sdk::nbgl::{Field, TagValueList};
        use crate::nbgl::{nbgl_review_fields, nbgl_sync_review_status, ReviewType};

        let fields = [Field{ name: "Hash", value: hex_str }];
        let values = TagValueList::new(&fields, 0, false, false);
        let approved = nbgl_review_fields("Review", "Hash", &values);
        if approved {
            nbgl_sync_review_status(ReviewType::Hash);
            sign_hash(path, message)
        } else {
            Err(ErrorCode::UserCancelled)
        }
    }
}

fn sign_hash(path: &[u32], message: &[u8]) -> Result<([u8; 72], u32, u32), ErrorCode> {
    Secp256k1::derive_from_path(path)
        .deterministic_sign(message)
        .map_err(|_| ErrorCode::TxSigningFailed)
}
