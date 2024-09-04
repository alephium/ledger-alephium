#![no_std]
#![no_main]

use crate::ui::tx_reviewer::TxReviewer;
use handler::{handle_apdu, Ins};
use ledger_device_sdk::io;
use sign_tx_context::SignTxContext;

mod blake2b_hasher;
mod debug;
mod error_code;
mod handler;
mod ledger_sdk_stub;
mod public_key;
mod settings;
mod sign_tx_context;
mod token_verifier;
mod ui;

ledger_device_sdk::set_panic!(ledger_device_sdk::exiting_panic);

// This function is the app entry point
#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();

    // Initialize the sign tx context and tx reviewer
    let mut sign_tx_context: SignTxContext = SignTxContext::new();
    let mut tx_reviewer: TxReviewer = TxReviewer::new();

    #[cfg(not(any(target_os = "stax", target_os = "flex")))]
    {
        use crate::ui::bagl::home::MainPages;

        let mut main_pages = MainPages::new();
        loop {
            // Wait for either a specific button push to exit the app
            // or an APDU command
            if let io::Event::Command(ins) = main_pages.show::<Ins>(&mut comm) {
                match handle_apdu(&mut comm, ins, &mut sign_tx_context, &mut tx_reviewer) {
                    Ok(()) => comm.reply_ok(),
                    Err(sw) => comm.reply(sw),
                }
                main_pages.show_ui();
            }
        }
    }

    #[cfg(any(target_os = "stax", target_os = "flex"))]
    {
        use crate::ledger_sdk_stub::nbgl_display::nbgl_display;
        use ledger_device_sdk::nbgl::init_comm;
        use ledger_secure_sdk_sys::INIT_HOME_PAGE;

        init_comm(&mut comm);
        let settings_strings = &[["Blind signing", "Enable blind signing"]];

        loop {
            let event = if tx_reviewer.display_settings() {
                nbgl_display::<Ins>(&mut comm, settings_strings, 0)
            } else if !tx_reviewer.review_started() {
                nbgl_display::<Ins>(&mut comm, settings_strings, INIT_HOME_PAGE as u8)
            } else {
                comm.next_event()
            };
            if let io::Event::Command(ins) = event {
                match handle_apdu(&mut comm, ins, &mut sign_tx_context, &mut tx_reviewer) {
                    Ok(_) => comm.reply_ok(),
                    Err(sw) => comm.reply(sw),
                }
            }
        }
    }
}
