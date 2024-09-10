use include_gif::include_gif;
use ledger_device_sdk::io::{self, ApduHeader, Reply};
use ledger_device_sdk::ui::{
    bagls,
    bitmaps::{Glyph, DASHBOARD_X},
    gadgets,
    layout::Draw,
    screen_util,
};
use ledger_secure_sdk_sys::buttons::ButtonEvent;

use crate::settings::{is_blind_signing_enabled, toggle_blind_signing_setting};

const UI_PAGE_NUM: u8 = 4;

fn show_ui_common(draw: fn() -> ()) {
    gadgets::clear_screen();

    bagls::LEFT_ARROW.display();
    bagls::RIGHT_ARROW.display();

    draw();

    screen_util::screen_update();
}

fn show_ui_welcome() {
    show_ui_common(|| {
        const APP_ICON: Glyph = Glyph::from_include(include_gif!("alph_14x14.gif"));
        gadgets::Page::from((["Alephium ", "is ready"], &APP_ICON)).place();
    });
}

fn show_ui_blind_signing() {
    show_ui_common(|| {
        let label = if is_blind_signing_enabled() {
            "enabled"
        } else {
            "disabled"
        };
        gadgets::Page::from((["Blind Signing", label], false)).place();
    });
}

fn show_ui_version() {
    show_ui_common(|| {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        gadgets::Page::from((["Version", VERSION], false)).place();
    });
}

fn show_ui_quit() {
    show_ui_common(|| {
        gadgets::Page::from(("Quit", &DASHBOARD_X)).place();
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

pub struct MainPages {
    ui_index: u8,
}

impl MainPages {
    pub fn new() -> Self {
        show_ui(0);
        MainPages { ui_index: 0 }
    }

    pub fn show_ui(&mut self) {
        show_ui(self.ui_index);
    }

    #[inline]
    fn right_page(&mut self) {
        self.ui_index = (self.ui_index + 1) % UI_PAGE_NUM;
        show_ui(self.ui_index);
    }

    #[inline]
    fn left_page(&mut self) {
        self.ui_index = (self.ui_index - 1) % UI_PAGE_NUM;
        show_ui(self.ui_index);
    }

    pub fn show<T>(&mut self, comm: &mut io::Comm) -> io::Event<T>
    where
        T: TryFrom<ApduHeader>,
        Reply: From<<T as TryFrom<ApduHeader>>::Error>,
    {
        loop {
            match comm.next_event() {
                io::Event::Button(ButtonEvent::LeftButtonPress) => {
                    bagls::LEFT_S_ARROW.instant_display();
                }
                io::Event::Button(ButtonEvent::RightButtonPress) => {
                    bagls::RIGHT_S_ARROW.instant_display();
                }
                io::Event::Button(ButtonEvent::RightButtonRelease) => {
                    self.right_page();
                }
                io::Event::Button(ButtonEvent::LeftButtonRelease) => {
                    self.left_page();
                }
                io::Event::Button(ButtonEvent::BothButtonsRelease) => {
                    if self.ui_index == 2 {
                        toggle_blind_signing_setting();
                        show_ui_blind_signing();
                    } else if self.ui_index == 3 {
                        ledger_device_sdk::exit_app(0);
                    }
                }
                event => return event,
            }
        }
    }
}
