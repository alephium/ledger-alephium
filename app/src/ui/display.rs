use ledger_device_sdk::io;
use ledger_device_sdk::ui::{
    bagls, gadgets,
    layout::{self, Draw, StringPlace},
    screen_util,
};
use ledger_secure_sdk_sys::buttons::ButtonEvent;

use crate::{
    settings::{is_blind_signing_enabled, update_blind_signing},
    Ins,
};

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

    pub fn show(&mut self, comm: &mut io::Comm) -> io::Event<Ins> {
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
                        update_blind_signing();
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
