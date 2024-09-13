use ledger_device_sdk::ui::{
    bagls::{self, Icon, Label},
    bitmaps::Glyph,
    fonts::OPEN_SANS,
    gadgets::{clear_screen, get_event},
    layout::{self, Draw, Layout, Location, StringPlace},
    screen_util::screen_update,
};
use ledger_secure_sdk_sys::buttons::{ButtonEvent, ButtonsState};
use numtoa::NumToA;

const MAX_CHAR_PER_LINE: usize = 17;

// The code is from ledger-rust-sdk: https://github.com/LedgerHQ/ledger-device-rust-sdk/blob/3c22c1c1b5e2d909e34409fc92cfeed775541a63/ledger_device_sdk/src/ui/gadgets.rs#L803.
// We've made modifications here to ensure the `Approve` page comes before the `Reject` page.
pub struct MultiFieldReview<'a> {
    fields: &'a [Field<'a>],
    review_message: &'a [&'a str],
    review_glyph: Option<&'a Glyph<'a>>,
    validation_message: [&'a str; 2],
    validation_glyph: Option<&'a Glyph<'a>>,
    cancel_message: &'a str,
    cancel_glyph: Option<&'a Glyph<'a>>,
}

impl<'a> MultiFieldReview<'a> {
    pub fn new(
        fields: &'a [Field<'a>],
        review_message: &'a [&'a str],
        review_glyph: Option<&'a Glyph<'a>>,
        validation_message: [&'a str; 2],
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

    pub fn simple(
        fields: &'a [Field<'a>],
        review_message: &'a [&'a str],
        review_glyph: Option<&'a Glyph<'a>>,
        validation_message: &'a str,
        validation_glyph: Option<&'a Glyph<'a>>,
        cancel_message: &'a str,
        cancel_glyph: Option<&'a Glyph<'a>>,
    ) -> Self {
        MultiFieldReview::new(
            fields,
            review_message,
            review_glyph,
            [validation_message, ""],
            validation_glyph,
            cancel_message,
            cancel_glyph,
        )
    }

    pub fn show(&self) -> bool {
        let first_page_opt = match self.review_message.len() {
            0 => None,
            1 => Some(Page::new(
                PageStyle::PictureBold,
                [self.review_message[0], ""],
                self.review_glyph,
            )),
            _ => Some(Page::new(
                PageStyle::PictureNormal,
                [self.review_message[0], self.review_message[1]],
                self.review_glyph,
            )),
        };

        display_first_page(&first_page_opt);

        let validation_page = Page::new(
            PageStyle::PictureBold,
            self.validation_message,
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
                    bagls::LEFT_ARROW.display();
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
                    bagls::LEFT_ARROW.display();
                    bagls::RIGHT_ARROW.display();
                    validation_page.place();
                    screen_update();
                    loop {
                        match get_event(&mut buttons) {
                            Some(ButtonEvent::LeftButtonRelease) => {
                                cur_page = cur_page.saturating_sub(1);
                                if cur_page == 0 && self.fields.is_empty() {
                                    display_first_page(&first_page_opt);
                                } else {
                                    direction = ButtonEvent::LeftButtonRelease;
                                }
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
                    direction = self.fields[cur_page]
                        .event_loop(direction, cur_page == 0 && first_page_opt.is_none());
                    match direction {
                        ButtonEvent::LeftButtonRelease => {
                            if cur_page == 0 {
                                display_first_page(&first_page_opt);
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

pub fn display_first_page(page_opt: &Option<Page>) {
    match page_opt {
        Some(page) => {
            clear_screen();
            bagls::RIGHT_ARROW.display();
            page.place();
            screen_update();

            let mut buttons = ButtonsState::new();
            loop {
                if let Some(ButtonEvent::RightButtonRelease) = get_event(&mut buttons) {
                    return;
                }
            }
        }
        None => (),
    }
}

pub struct Field<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

impl<'a> Field<'a> {
    pub fn event_loop(&self, incoming_direction: ButtonEvent, is_first_field: bool) -> ButtonEvent {
        let mut buttons = ButtonsState::new();
        let chunk_max_lines = layout::MAX_LINES - 1;
        let max_size_per_page = chunk_max_lines * MAX_CHAR_PER_LINE;
        let page_count = (self.value.len() + max_size_per_page - 1) / max_size_per_page;

        let mut cur_page = match incoming_direction {
            ButtonEvent::LeftButtonRelease => page_count - 1,
            ButtonEvent::RightButtonRelease => 0,
            _ => 0,
        };

        // A closure to draw common elements of the screen
        // cur_page passed as parameter to prevent borrowing
        let draw = |page: usize| {
            clear_screen();
            let mut chunks = [Label::default(); layout::MAX_LINES];
            for (i, chunk) in self
                .value
                .as_bytes()
                .chunks(MAX_CHAR_PER_LINE)
                .skip(page * chunk_max_lines)
                .take(chunk_max_lines)
                .enumerate()
            {
                chunks[1 + i] = Label::from(core::str::from_utf8(chunk).unwrap_or(""));
            }

            let mut header_buf = [b' '; MAX_CHAR_PER_LINE + 4];

            if page == 0 && MAX_CHAR_PER_LINE * chunk_max_lines > self.value.len() {
                // There is a single page. Do not display counter `( x / n )`
                header_buf[..self.name.len()].copy_from_slice(self.name.as_bytes());
            } else {
                let mut buf_page = [0u8; 3];
                let mut buf_count = [0u8; 3];
                let page_str = (page + 1).numtoa_str(10, &mut buf_page);
                let count_str = page_count.numtoa_str(10, &mut buf_count);

                concatenate(
                    &[&self.name, " (", &page_str, "/", &count_str, ")"],
                    &mut header_buf,
                );
            }
            let header = core::str::from_utf8(&header_buf)
                .unwrap_or("")
                .trim_end_matches(' ');
            chunks[0] = Label::from(header).bold();

            if !is_first_field {
                bagls::LEFT_ARROW.display();
            }
            bagls::RIGHT_ARROW.display();

            chunks.place(Location::Middle, Layout::Centered, false);

            screen_update();
        };

        draw(cur_page);

        loop {
            match get_event(&mut buttons) {
                Some(ButtonEvent::LeftButtonRelease) => {
                    if cur_page == 0 {
                        return ButtonEvent::LeftButtonRelease;
                    }
                    cur_page = cur_page.saturating_sub(1);
                    draw(cur_page);
                }
                Some(ButtonEvent::RightButtonRelease) => {
                    if cur_page + 1 == page_count {
                        return ButtonEvent::RightButtonRelease;
                    }
                    if cur_page + 1 < page_count {
                        cur_page += 1;
                    }
                    draw(cur_page);
                }
                Some(_) | None => (),
            }
        }
    }
}

// Function to concatenate multiple strings into a fixed-size array
fn concatenate(strings: &[&str], output: &mut [u8]) {
    let mut offset = 0;

    for s in strings {
        let s_len = s.len();
        let copy_len = core::cmp::min(s_len, output.len() - offset);

        if copy_len > 0 {
            output[offset..offset + copy_len].copy_from_slice(&s.as_bytes()[..copy_len]);
            offset += copy_len;
        } else {
            // If the output buffer is full, stop concatenating.
            break;
        }
    }
}

// This is a modified version of the Ledger Rust SDK code.
// The modifications were made to address the issue of the Ledger Rust SDK
// not supporting two lines of bold text.
#[derive(Copy, Clone, PartialEq, Default)]
pub enum PageStyle {
    #[default]
    PictureNormal, // Picture (should be 16x16) with two lines of text (page layout depends on device).
    PictureBold, // Icon on top with one line of text on the bottom.
    BoldNormal,  // One line of bold text and one line of normal text.
    Normal,      // 2 lines of centered text.
}

#[derive(Copy, Clone, Default)]
pub struct Page<'a> {
    style: PageStyle,
    label: [&'a str; 2],
    glyph: Option<&'a Glyph<'a>>,
}

// new_picture_normal
impl<'a> From<([&'a str; 2], &'a Glyph<'a>)> for Page<'a> {
    fn from((label, glyph): ([&'a str; 2], &'a Glyph<'a>)) -> Page<'a> {
        Page::new(PageStyle::PictureNormal, [label[0], label[1]], Some(glyph))
    }
}

// new bold normal or new normal
impl<'a> From<([&'a str; 2], bool)> for Page<'a> {
    fn from((label, bold): ([&'a str; 2], bool)) -> Page<'a> {
        if bold {
            Page::new(PageStyle::BoldNormal, [label[0], label[1]], None)
        } else {
            Page::new(PageStyle::Normal, [label[0], label[1]], None)
        }
    }
}

// new picture bold
impl<'a> From<(&'a str, &'a Glyph<'a>)> for Page<'a> {
    fn from((label, glyph): (&'a str, &'a Glyph<'a>)) -> Page<'a> {
        let label = [label, ""];
        Page::new(PageStyle::PictureBold, label, Some(glyph))
    }
}

impl<'a> Page<'a> {
    pub const fn new(style: PageStyle, label: [&'a str; 2], glyph: Option<&'a Glyph<'a>>) -> Self {
        Page {
            style,
            label,
            glyph,
        }
    }

    pub fn place(&self) {
        match self.style {
            PageStyle::Normal => {
                self.label.place(Location::Middle, Layout::Centered, false);
            }
            PageStyle::PictureNormal => {
                let icon_x = 57;
                let icon_y = 10;
                self.label
                    .place(Location::Custom(28), Layout::Centered, false);
                if let Some(glyph) = self.glyph {
                    let icon = Icon::from(glyph);
                    icon.set_x(icon_x).set_y(icon_y).display();
                }
            }
            PageStyle::PictureBold => {
                let icon_x = 57;
                let icon_y = 10;
                self.label[0].place(Location::Custom(28), Layout::Centered, true);
                self.label[1].place(Location::Custom(42), Layout::Centered, true);
                if let Some(glyph) = self.glyph {
                    let icon = Icon::from(glyph);
                    icon.set_x(icon_x).set_y(icon_y).display();
                }
            }
            PageStyle::BoldNormal => {
                let padding = 1;
                let max_text_lines = 3;
                let total_height = (OPEN_SANS[0].height * max_text_lines) as usize
                    + OPEN_SANS[1].height as usize
                    + 2 * padding as usize;
                let mut cur_y = Location::Middle.get_y(total_height);

                self.label[0].place(Location::Custom(cur_y), Layout::Centered, true);
                cur_y += OPEN_SANS[0].height as usize + 2 * padding as usize;

                // Display the second label as up to 3 lines of text
                let mut indices = [(0, 0); 3];
                let len = self.label[1].len();
                for (i, indice) in indices.iter_mut().enumerate() {
                    let start = (i * MAX_CHAR_PER_LINE).min(len);
                    if start >= len {
                        break; // Break if we reach the end of the string
                    }
                    let end = (start + MAX_CHAR_PER_LINE).min(len);
                    *indice = (start, end);
                    (&self.label[1][start..end]).place(
                        Location::Custom(cur_y),
                        Layout::Centered,
                        false,
                    );
                    cur_y += OPEN_SANS[0].height as usize + 2 * padding as usize;
                }
            }
        }
    }
}
