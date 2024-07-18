use ledger_device_sdk::ui::{
    bitmaps::Glyph,
    gadgets::{clear_screen, get_event, Field, Page, PageStyle},
    screen_util::screen_update,
};
use ledger_secure_sdk_sys::buttons::{ButtonEvent, ButtonsState};

// The code is from ledger-rust-sdk: https://github.com/LedgerHQ/ledger-device-rust-sdk/blob/3c22c1c1b5e2d909e34409fc92cfeed775541a63/ledger_device_sdk/src/ui/gadgets.rs#L803.
// We've made modifications here to ensure the `Approve` page comes before the `Reject` page.
pub struct MultiFieldReview<'a> {
    fields: &'a [Field<'a>],
    review_message: &'a [&'a str],
    review_glyph: Option<&'a Glyph<'a>>,
    validation_message: &'a str,
    validation_glyph: Option<&'a Glyph<'a>>,
    cancel_message: &'a str,
    cancel_glyph: Option<&'a Glyph<'a>>,
}

impl<'a> MultiFieldReview<'a> {
    pub fn new(
        fields: &'a [Field<'a>],
        review_message: &'a [&'a str],
        review_glyph: Option<&'a Glyph<'a>>,
        validation_message: &'a str,
        validation_glyph: Option<&'a Glyph<'a>>,
        cancel_message: &'a str,
        cancel_glyph: Option<&'a Glyph<'a>>,
    ) -> Self {
        MultiFieldReview {
            fields,
            review_message,
            review_glyph,
            validation_message,
            validation_glyph,
            cancel_message,
            cancel_glyph,
        }
    }

    pub fn show(&self) -> bool {
        let first_page = match self.review_message.len() {
            0 => Page::new(PageStyle::PictureNormal, ["", ""], self.review_glyph),
            1 => Page::new(
                PageStyle::PictureBold,
                [self.review_message[0], ""],
                self.review_glyph,
            ),
            _ => Page::new(
                PageStyle::PictureNormal,
                [self.review_message[0], self.review_message[1]],
                self.review_glyph,
            ),
        };

        clear_screen();
        first_page.place_and_wait();
        screen_update();

        let validation_page = Page::new(
            PageStyle::PictureBold,
            [self.validation_message, ""],
            self.validation_glyph,
        );
        let cancel_page = Page::new(
            PageStyle::PictureBold,
            [self.cancel_message, ""],
            self.cancel_glyph,
        );

        let mut cur_page = 0usize;
        let mut direction = ButtonEvent::RightButtonRelease;

        loop {
            match cur_page {
                cancel if cancel == self.fields.len() + 1 => {
                    let mut buttons = ButtonsState::new();
                    clear_screen();
                    cancel_page.place();
                    screen_update();
                    loop {
                        match get_event(&mut buttons) {
                            Some(ButtonEvent::LeftButtonRelease) => {
                                cur_page = cur_page.saturating_sub(1);
                                break;
                            }
                            Some(ButtonEvent::BothButtonsRelease) => return false,
                            _ => (),
                        }
                    }
                }
                validation if validation == self.fields.len() => {
                    let mut buttons = ButtonsState::new();
                    clear_screen();
                    validation_page.place();
                    screen_update();
                    loop {
                        match get_event(&mut buttons) {
                            Some(ButtonEvent::LeftButtonRelease) => {
                                cur_page = cur_page.saturating_sub(1);
                                break;
                            }
                            Some(ButtonEvent::RightButtonRelease) => {
                                cur_page += 1;
                                break;
                            }
                            Some(ButtonEvent::BothButtonsRelease) => return true,
                            _ => (),
                        }
                    }
                }
                _ => {
                    direction = self.fields[cur_page].event_loop(direction);
                    match direction {
                        ButtonEvent::LeftButtonRelease => {
                            if cur_page == 0 {
                                direction = ButtonEvent::RightButtonRelease;
                            } else {
                                cur_page -= 1;
                            }
                        }
                        ButtonEvent::RightButtonRelease => {
                            cur_page += 1;
                        }
                        _ => (),
                    }
                }
            }
        }
    }
}