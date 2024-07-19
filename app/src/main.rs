#![no_std]
#![no_main]

#[cfg(not(any(target_os = "stax", target_os = "flex")))]
use crate::ui::display::MainPages;
use handler::{handle_apdu, Ins};
use ledger_device_sdk::io;
#[cfg(any(target_os = "stax", target_os = "flex"))]
use ledger_device_sdk::nbgl::{NbglHomeAndSettings, init_comm};
#[cfg(any(target_os = "stax", target_os = "flex"))]
use settings::SETTINGS_DATA;
use sign_tx_context::SignTxContext;
use crate::ui::tx_reviewer::TxReviewer;

mod blake2b_hasher;
mod settings;
mod debug;
mod error_code;
mod public_key;
mod sign_tx_context;
mod ledger_sdk_stub;
mod ui;
mod handler;

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
