#![no_std]
#![no_main]

use blind_signing::is_blind_signing_enabled;
use blind_signing::update_blind_signing;
use debug::print::println_array;
use error_code::ErrorCode;
use ledger_device_sdk::ui::layout;
use ledger_device_sdk::ui::layout::Draw;
use ledger_device_sdk::ui::layout::StringPlace;
use ledger_secure_sdk_sys::buttons::ButtonEvent;
use public_key::derive_pub_key;
use sign_tx_context::SignTxContext;
use tx_reviewer::TxReviewer;
use utils::{self, deserialize_path};
mod blake2b_hasher;
mod blind_signing;
mod debug;
mod error_code;
mod nvm_buffer;
mod public_key;
mod sign_tx_context;
mod tx_reviewer;

use debug::print::{println, println_slice};
use ledger_device_sdk::io;
use ledger_device_sdk::ui::bagls;
use ledger_device_sdk::ui::gadgets;
use ledger_device_sdk::ui::screen_util;

ledger_device_sdk::set_panic!(ledger_device_sdk::exiting_panic);

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
            let (pk, hd_index) = derive_pub_key(&mut path, p1, p2)?;

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
