#![no_std]
#![no_main]

use nanos_sdk::buttons::ButtonEvent;
use nanos_ui::layout;
use nanos_ui::layout::Draw;
use nanos_ui::layout::StringPlace;
use utils::{self, deserialize_path, djb_hash, xor_bytes};
mod app_utils;

use app_utils::*;
use app_utils::print::{println, println_array, println_slice};
use core::str::from_utf8;
use nanos_sdk::ecc::{Secp256k1, ECPublicKey};
use nanos_sdk::io;
use nanos_sdk::io::SyscallError;
use nanos_ui::ui;
use nanos_ui::bagls;
use nanos_ui::screen_util;

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

/// This is the UI flow for signing, composed of a scroller
/// to read the incoming message, a panel that requests user
/// validation, and an exit message.
fn sign_ui(path: &[u32], message: &[u8]) -> Result<Option<([u8; 72], u32)>, SyscallError> {
    ui::popup("Tx hash review:");

    {
        let hex: [u8; 64] = utils::to_hex(message).map_err(|_| SyscallError::Overflow)?;
        let m = from_utf8(&hex).map_err(|_| SyscallError::InvalidParameter)?;

        ui::MessageScroller::new(m).event_loop();
    }

    if ui::Validator::new("Sign ?").ask() {
        let signature = Secp256k1::from_bip32(path)
            .deterministic_sign(message)
            .map_err(|_| SyscallError::Unspecified)?;
        ui::SingleMessage::new("Signing...").show();
        Ok(Some(signature))
    } else {
        Ok(None)
    }
}

fn show_ui_common(draw: fn() -> ()) {
    ui::clear_screen();

    bagls::LEFT_ARROW.display();
    bagls::RIGHT_ARROW.display();

    draw();

    screen_util::screen_update();
}

fn show_ui_welcome() {
    show_ui_common(||{
        let mut lines = [bagls::Label::from_const("Alephium"), bagls::Label::from_const("ready")];
        lines[0].bold = true;
        lines.place(layout::Location::Middle, layout::Layout::Centered, false);
    });
}

fn show_ui_version() {
    show_ui_common(|| {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        let mut lines = [bagls::Label::from_const("Version"), bagls::Label::from_const(VERSION)];
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
        2 => show_ui_quit(),
        _ => panic!("Invalid ui index")
    }
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();
    let mut ui_index = 0;
    let ui_page_num = 3;

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
                ui_index = (ui_index + ui_page_num - 1 ) % ui_page_num;
                show_ui(ui_index);
            }
            io::Event::Button(ButtonEvent::BothButtonsRelease) => {
                if ui_index == 2 {
                    nanos_sdk::exit_app(0);
                }
            }
            io::Event::Command(ins) => {
                println("=== Before event");
                println_array::<1, 2>(&[ui_index]);
                match handle_apdu(&mut comm, ins) {
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
            }
            _ => (),
        }
    }
}

#[repr(u8)]
enum Ins {
    GetVersion,
    GetPubKey,
    SignHash,
}

impl From<u8> for Ins {
    fn from(ins: u8) -> Ins {
        match ins {
            0 => Ins::GetVersion,
            1 => Ins::GetPubKey,
            2 => Ins::SignHash,
            _ => panic!(),
        }
    }
}

use nanos_sdk::io::Reply;

fn handle_apdu(comm: &mut io::Comm, ins: Ins) -> Result<bool, Reply> {
    if comm.rx == 0 {
        return Err(io::StatusWords::NothingReceived.into());
    }

    let mut path: [u32; 5] = [0; 5];

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
                return Err(io::StatusWords::BadLen.into());
            }
            println_slice::<40>(raw_path);

            let p1 = comm.get_p1();
            let p2 = comm.get_p2();

            let (pk, hd_index) = if p1 == 0 {
                (derive_pub_key(& mut path)?, path[path.len() - 1])
            } else {
                let group_num = p1;
                let target_group = p2 % p1;
                assert!(target_group < group_num);
                derive_pub_key_for_group(& mut path, group_num, target_group)?
            };

            println_slice::<130>(pk.as_ref());
            comm.append(pk.as_ref());
            comm.append(hd_index.to_be_bytes().as_slice());
        }
        Ins::SignHash => {
            let data = comm.get_data()?;
            if data.len() != 4 * 5 + 32 {
                return Err(io::StatusWords::BadLen.into());
            }
            // This check can be removed, but we keep it for double checking
            if !deserialize_path(&data[..20], &mut path) {
                return Err(io::StatusWords::BadLen.into());
            }

            let out = sign_ui(&path, &data[20..])?;
            if let Some((signature_buf, length)) = out {
                comm.append(&signature_buf[..length as usize])
            }
            return Ok(true)
        }
    }
    Ok(false)
}

fn derive_pub_key(path: &[u32]) -> Result<ECPublicKey<65, 'W'>, Reply> {
    let pk = Secp256k1::from_bip32(path)
        .public_key()
        .map_err(|x| Reply(0x6eu16 | (x as u16 & 0xff)))?;
    return Ok(pk);
}

fn derive_pub_key_for_group(path: &mut [u32], group_num: u8, target_group: u8) -> Result<(ECPublicKey<65, 'W'>, u32), Reply> {
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
    let mut compressed = [0 as u8; 33];
    compressed[1..33].copy_from_slice(&pub_key[1..33]);
    if pub_key.last().unwrap() % 2 == 0 {
        compressed[0] = 0x02
    } else {
        compressed[0] = 0x03
    }
    println("compressed");
    println_slice::<66>(&compressed);

    let pub_key_hash = blake2b(&compressed);
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

    return group_index % group_num;
}
