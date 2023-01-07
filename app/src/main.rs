#![no_std]
#![no_main]

use utils::{self, deserialize_path};
mod app_utils;

use core::str::from_utf8;
use nanos_sdk::ecc::Secp256k1;
use nanos_sdk::io;
use nanos_sdk::io::SyscallError;
use nanos_ui::ui;
use app_utils::print::{println, println_slice};

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

/// Display public key in two separate
/// message scrollers
// fn show_pubkey(path: &[u32]) {
//     let pubkey = Secp256k1::from_bip32(path).public_key();
//     match pubkey {
//         Ok(pk) => {
//             {
//                 let hex0: [u8; 64] = utils::to_hex(&pk.as_ref()[1..33]).unwrap();
//                 let m = from_utf8(&hex0).unwrap();
//                 ui::MessageScroller::new(m).event_loop();
//             }
//             {
//                 let hex1: [u8; 64] = utils::to_hex(&pk.as_ref()[33..65]).unwrap();
//                 let m = from_utf8(&hex1).unwrap();
//                 ui::MessageScroller::new(m).event_loop();
//             }
//         }
//         Err(_) => ui::popup("Error"),
//     }
// }

/// Basic nested menu. Will be subject
/// to simplifications in the future.
// #[allow(clippy::needless_borrow)]
// fn menu_example() {
//     loop {
//         match ui::Menu::new(&[&"PubKey", &"Infos", &"Back", &"Exit App"]).show() {
//             0 => show_pubkey(),
//             1 => loop {
//                 match ui::Menu::new(&[&"Copyright", &"Authors", &"Back"]).show() {
//                     0 => ui::popup("2020 Ledger"),
//                     1 => ui::popup("???"),
//                     _ => break,
//                 }
//             },
//             2 => return,
//             3 => nanos_sdk::exit_app(0),
//             _ => (),
//         }
//     }
// }

/// This is the UI flow for signing, composed of a scroller
/// to read the incoming message, a panel that requests user
/// validation, and an exit message.
fn sign_ui(path: &[u32], message: &[u8]) -> Result<Option<([u8; 72], u32)>, SyscallError> {
    ui::popup("Message review");

    {
        let hex: [u8; 64] = utils::to_hex(message).map_err(|_| SyscallError::Overflow)?;
        let m = from_utf8(&hex).map_err(|_| SyscallError::InvalidParameter)?;

        ui::MessageScroller::new(m).event_loop();
    }

    if ui::Validator::new("Sign ?").ask() {
        let signature = Secp256k1::from_bip32(path)
            .deterministic_sign(message)
            .map_err(|_| SyscallError::Unspecified)?;
        ui::popup("Done !");
        Ok(Some(signature))
    } else {
        ui::popup("Cancelled");
        Ok(None)
    }
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();

    // Draw some 'welcome' screen
    ui::SingleMessage::new("A l e p h i u m").show();

    loop {

        // Wait for either a specific button push to exit the app
        // or an APDU command
        match comm.next_event() {
            // io::Event::Button(ButtonEvent::RightButtonRelease) => nanos_sdk::exit_app(0),
            io::Event::Command(ins) => {
                println("Event");
                match handle_apdu(&mut comm, ins) {
                    Ok(()) => comm.reply_ok(),
                    Err(sw) => comm.reply(sw),
                }
            },
            _ => (),
        }
    }
}

#[repr(u8)]
enum Ins {
    GetPubkey,
    SignHash
}

impl From<u8> for Ins {
    fn from(ins: u8) -> Ins {
        match ins {
            0 => Ins::GetPubkey,
            1 => Ins::SignHash,
            _ => panic!(),
        }
    }
}

use nanos_sdk::io::Reply;

fn handle_apdu(comm: &mut io::Comm, ins: Ins) -> Result<(), Reply> {
    if comm.rx == 0 {
        return Err(io::StatusWords::NothingReceived.into());
    }

    let mut path: [u32; 5] = [0; 5];

    match ins {
        Ins::GetPubkey => {
            let raw_path = comm.get_data()?;
            if !deserialize_path(raw_path, &mut path) {
                return Err(io::StatusWords::BadLen.into())
            }

            println_slice::<40>(raw_path);

            let pk = Secp256k1::from_bip32(&mut path)
                .public_key()
                .map_err(|x| Reply(0x6eu16 | (x as u16 & 0xff)))?;
            
            println_slice::<130>(pk.as_ref());
            comm.append(pk.as_ref());
        }
        Ins::SignHash => {
            let data = comm.get_data()?;
            if data.len() != 4*5 + 32 {
                return Err(io::StatusWords::BadLen.into())
            }
            // This check can be removed, but we keep it for double checking
            if !deserialize_path(&data[..20], &mut path) {
                return Err(io::StatusWords::BadLen.into())
            }

            let out = sign_ui(&path, &data[20..])?;
            if let Some((signature_buf, length)) = out {
                comm.append(&signature_buf[..length as usize])
            }
        }
    }
    Ok(())
}
