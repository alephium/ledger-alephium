#![no_std]
#![no_main]

use blind_signing::is_blind_signing_enabled;
use blind_signing::update_blind_signing;
use error_code::ErrorCode;
use ledger_device_sdk::ecc::SeedDerive;
use ledger_device_sdk::ui::layout;
use ledger_device_sdk::ui::layout::Draw;
use ledger_device_sdk::ui::layout::StringPlace;
use ledger_secure_sdk_sys::buttons::ButtonEvent;
use sign_tx_context::SignTxContext;
use tx_reviewer::TxReviewer;
use utils::{self, deserialize_path, djb_hash, xor_bytes};
mod blake2b_hasher;
mod blind_signing;
mod debug;
mod error_code;
mod nvm_buffer;
mod sign_tx_context;
mod tx_reviewer;

use debug::print::{println, println_array, println_slice};
use ledger_device_sdk::ecc::{ECPublicKey, Secp256k1};
use ledger_device_sdk::io;
use ledger_device_sdk::ui::bagls;
use ledger_device_sdk::ui::gadgets;
use ledger_device_sdk::ui::screen_util;

ledger_device_sdk::set_panic!(ledger_device_sdk::exiting_panic);

const TOTAL_NUMBER_OF_GROUPS: u8 = 4;

fn show_ui_common(draw: fn() -> ()) {
    gadgets::clear_screen();

    bagls::LEFT_ARROW.display();
    bagls::RIGHT_ARROW.display();

    draw();

    screen_util::screen_update();
}

fn show_ui_welcome() {
    show_ui_common(|| {
        let mut lines = [
            bagls::Label::from_const("Alephium"),
            bagls::Label::from_const("ready"),
        ];
        lines[0].bold = true;
        lines.place(layout::Location::Middle, layout::Layout::Centered, false);
    });
}

fn show_ui_blind_signing() {
    show_ui_common(|| {
        let mut lines = [
            bagls::Label::from_const("Blind Signing"),
            bagls::Label::from_const(if is_blind_signing_enabled() {
                "enabled"
            } else {
                "disabled"
            }),
        ];
        lines[0].bold = true;
        lines.place(layout::Location::Middle, layout::Layout::Centered, false);
    });
}

fn show_ui_version() {
    show_ui_common(|| {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        let mut lines = [
            bagls::Label::from_const("Version"),
            bagls::Label::from_const(VERSION),
        ];
        lines[0].bold = true;
        lines.place(layout::Location::Middle, layout::Layout::Centered, false);
    });
}

fn show_ui_quit() {
    show_ui_common(|| {
        let mut lines = [bagls::Label::from_const("Quit")];
        lines[0].bold = true;
        lines.place(layout::Location::Middle, layout::Layout::Centered, false);
    });
}

fn show_ui(index: u8) {
    match index {
        0 => show_ui_welcome(),
        1 => show_ui_version(),
        2 => show_ui_blind_signing(),
        3 => show_ui_quit(),
        _ => panic!("Invalid ui index"),
    }
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();
    let mut ui_index = 0;
    let ui_page_num = 4;

    let mut sign_tx_context: SignTxContext = SignTxContext::new();
    let mut tx_reviewer: TxReviewer = TxReviewer::new();
    // Draw some 'welcome' screen
    show_ui(ui_index);

    loop {
        // Wait for either a specific button push to exit the app
        // or an APDU command
        match comm.next_event() {
            io::Event::Button(ButtonEvent::LeftButtonPress) => {
                bagls::LEFT_S_ARROW.instant_display();
            }
            io::Event::Button(ButtonEvent::RightButtonPress) => {
                bagls::RIGHT_S_ARROW.instant_display();
            }
            io::Event::Button(ButtonEvent::RightButtonRelease) => {
                ui_index = (ui_index + 1) % ui_page_num;
                show_ui(ui_index);
            }
            io::Event::Button(ButtonEvent::LeftButtonRelease) => {
                ui_index = (ui_index + ui_page_num - 1) % ui_page_num;
                show_ui(ui_index);
            }
            io::Event::Button(ButtonEvent::BothButtonsRelease) => {
                if ui_index == 2 {
                    update_blind_signing();
                    show_ui_blind_signing();
                }
                if ui_index == 3 {
                    ledger_device_sdk::exit_app(0);
                }
            }
            io::Event::Command(ins) => {
                println("=== Before event");
                println_array::<1, 2>(&[ui_index]);
                match handle_apdu(&mut comm, ins, &mut sign_tx_context, &mut tx_reviewer) {
                    Ok(ui_changed) => {
                        comm.reply_ok();
                        if ui_changed {
                            ui_index = 0;
                            show_ui(ui_index);
                        }
                    }
                    Err(sw) => comm.reply(sw),
                }
                println("=== After event");
                println_array::<1, 2>(&[ui_index]);
                show_ui(ui_index);
            }
            _ => (),
        }
    }
}

#[repr(u8)]
enum Ins {
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

use ledger_device_sdk::io::Reply;

use crate::blake2b_hasher::Blake2bHasher;

fn check_group(p1: u8, p2: u8) -> Result<(), Reply> {
    if p1 == 0 && p2 == 0 {
        return Ok(());
    }
    if p2 >= p1 || p1 != TOTAL_NUMBER_OF_GROUPS {
        return Err(ErrorCode::BadP1P2.into());
    }
    return Ok(());
}

fn handle_apdu(
    comm: &mut io::Comm,
    ins: Ins,
    sign_tx_context: &mut SignTxContext,
    tx_reviewer: &mut TxReviewer,
) -> Result<bool, Reply> {
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
            println_slice::<40>(raw_path);

            let p1 = apdu_header.p1;
            let p2 = apdu_header.p2;
            check_group(p1, p2)?;

            let (pk, hd_index) = if p1 == 0 {
                (derive_pub_key(&path)?, path[path.len() - 1])
            } else {
                let group_num = p1;
                let target_group = p2 % p1;
                assert!(target_group < group_num);
                derive_pub_key_for_group(&mut path, group_num, target_group)?
            };

            println_slice::<130>(pk.as_ref());
            comm.append(pk.as_ref());
            comm.append(hd_index.to_be_bytes().as_slice());
        }
        Ins::SignTx => {
            let data = comm.get_data()?;
            match sign_tx_context.handle_data(apdu_header, data, tx_reviewer) {
                Ok(()) if !sign_tx_context.is_complete() => {
                    return Ok(false);
                }
                Ok(()) => {
                    let result = match sign_tx_context.review_tx_id_and_sign() {
                        Ok((signature_buf, length, _)) => {
                            comm.append(&signature_buf[..length as usize]);
                            Ok(true)
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
    Ok(false)
}

fn derive_pub_key(path: &[u32]) -> Result<ECPublicKey<65, 'W'>, Reply> {
    let pk = Secp256k1::derive_from_path(path)
        .public_key()
        .map_err(|x| Reply(0x6eu16 | (x as u16 & 0xff)))?;
    Ok(pk)
}

fn derive_pub_key_for_group(
    path: &mut [u32],
    group_num: u8,
    target_group: u8,
) -> Result<(ECPublicKey<65, 'W'>, u32), Reply> {
    loop {
        println("path");
        println_slice::<8>(&path.last().unwrap().to_be_bytes());
        let pk = derive_pub_key(path)?;
        if get_pub_key_group(pk.as_ref(), group_num) == target_group {
            return Ok((pk, path[path.len() - 1]));
        }
        path[path.len() - 1] += 1;
    }
}

pub fn get_pub_key_group(pub_key: &[u8], group_num: u8) -> u8 {
    assert!(pub_key.len() == 65);
    println("pub_key 65");
    println_slice::<130>(pub_key);
    let mut compressed = [0_u8; 33];
    compressed[1..33].copy_from_slice(&pub_key[1..33]);
    if pub_key.last().unwrap() % 2 == 0 {
        compressed[0] = 0x02
    } else {
        compressed[0] = 0x03
    }
    println("compressed");
    println_slice::<66>(&compressed);

    let pub_key_hash = Blake2bHasher::hash(&compressed).unwrap();
    println("blake2b done");
    let script_hint = djb_hash(&pub_key_hash) | 1;
    println("hint done");
    let group_index = xor_bytes(script_hint);
    println("pub key hash");
    println_slice::<64>(&pub_key_hash);
    println("script hint");
    println_slice::<8>(&script_hint.to_be_bytes());
    println("group index");
    println_slice::<2>(&[group_index]);

    group_index % group_num
}
