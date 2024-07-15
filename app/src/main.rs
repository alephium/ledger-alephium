#![no_std]
#![no_main]

use debug::print::println_array;
use display::MainPages;
use error_code::ErrorCode;
use public_key::derive_pub_key;
use sign_tx_context::SignTxContext;
use tx_reviewer::TxReviewer;
use utils::{self, deserialize_path};

mod blake2b_hasher;
mod blind_signing;
mod debug;
#[cfg(not(any(target_os = "stax", target_os = "flex")))]
mod display;
mod error_code;
mod nvm;
mod public_key;
mod sign_tx_context;
mod swapping_buffer;
mod tx_reviewer;

use debug::print::{println, println_slice};
use ledger_device_sdk::io;

ledger_device_sdk::set_panic!(ledger_device_sdk::exiting_panic);

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();
    let mut main_pages = MainPages::new();

    let mut sign_tx_context: SignTxContext = SignTxContext::new();
    let mut tx_reviewer: TxReviewer = TxReviewer::new();

    loop {
        // Wait for either a specific button push to exit the app
        // or an APDU command
        match main_pages.show(&mut comm) {
            io::Event::Command(ins) => {
                println("=== Before event");
                println_array::<1, 2>(&[main_pages.ui_index]);
                match handle_apdu(&mut comm, ins, &mut sign_tx_context, &mut tx_reviewer) {
                    Ok(()) => comm.reply_ok(),
                    Err(sw) => comm.reply(sw),
                }
                println("=== After event");
                println_array::<1, 2>(&[main_pages.ui_index]);
                main_pages.show_ui(main_pages.ui_index);
            }
            _ => ()
        }
    }
}

#[repr(u8)]
pub enum Ins {
    GetVersion,
    GetPubKey,
    SignTx,
}

impl TryFrom<io::ApduHeader> for Ins {
    type Error = ErrorCode;
    fn try_from(header: io::ApduHeader) -> Result<Self, Self::Error> {
        match header.ins {
            0 => Ok(Ins::GetVersion),
            1 => Ok(Ins::GetPubKey),
            2 => Ok(Ins::SignTx),
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
