#![no_std]
#![no_main]

use crate::ui::tx_reviewer::TxReviewer;
use handler::handle_apdu;
use ledger_device_sdk::io;
use sign_tx_context::SignTxContext;

mod blake2b_hasher;
mod debug;
mod error_code;
mod handler;
mod nvm;
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
        use handler::Ins;

        let mut main_pages = MainPages::new();
        loop {
            // Wait for either a specific button push to exit the app
            // or an APDU command
            if let io::Event::Command(ins) = main_pages.show::<Ins>(&mut comm) {
                match handle_apdu(&mut comm, ins, &mut sign_tx_context, &mut tx_reviewer) {
                    Ok(_) => comm.reply_ok(),
                    Err(sw) => comm.reply(sw),
                }
                main_pages.show_ui();
            }
        }
    }

    #[cfg(any(target_os = "stax", target_os = "flex"))]
    {
        use crate::settings::SETTINGS_DATA;
        use include_gif::include_gif;
        use ledger_device_sdk::nbgl::init_comm;
        use ledger_device_sdk::nbgl::{NbglGlyph, NbglHomeAndSettings, PageIndex};

        const APP_ICON: NbglGlyph = NbglGlyph::from_include(include_gif!("alph_64x64.gif", NBGL));
        let settings_strings: &[[&str; 2]] = &[["Blind signing", "Enable blind signing"]];
        let mut home_and_settings = NbglHomeAndSettings::new()
            .glyph(&APP_ICON)
            .settings(unsafe { SETTINGS_DATA.get_mut() }, settings_strings)
            .infos(
                "Alephium",
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_AUTHORS"),
            );

        init_comm(&mut comm);
        home_and_settings.show_and_return();

        loop {
            if let io::Event::Command(ins) = comm.next_event() {
                let display_home =
                    match handle_apdu(&mut comm, ins, &mut sign_tx_context, &mut tx_reviewer) {
                        Ok(result) => {
                            comm.reply_ok();
                            result
                        }
                        Err(sw) => {
                            comm.reply(sw);
                            true
                        }
                    };
                if tx_reviewer.display_settings() {
                    tx_reviewer.reset_display_settings();
                    home_and_settings = home_and_settings.set_start_page(PageIndex::Settings(0));
                    home_and_settings.show_and_return();
                } else if display_home {
                    home_and_settings = home_and_settings.set_start_page(PageIndex::Home);
                    home_and_settings.show_and_return();
                }
            }
        }
    }
}
