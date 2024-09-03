pub mod home;
pub mod tx_reviewer_inner;

use crate::{
    error_code::ErrorCode,
    ledger_sdk_stub::multi_field_review::{Field, MultiFieldReview},
    public_key::sign_hash,
    settings::is_blind_signing_enabled,
    ui::TxReviewerInner,
};
use core::str::from_utf8;
use ledger_device_sdk::{
    buttons::{ButtonEvent, ButtonsState},
    ui::{
        bitmaps::{CHECKMARK, CROSS, CROSSMARK, EYE},
        gadgets::{clear_screen, get_event, Page, PageStyle},
        screen_util::screen_update,
    },
};

pub fn sign_hash_ui(path: &[u32], message: &[u8]) -> Result<([u8; 72], u32, u32), ErrorCode> {
    let hex: [u8; 64] = utils::to_hex(message).ok_or(ErrorCode::BadLen)?;
    let hex_str = from_utf8(&hex).map_err(|_| ErrorCode::InternalError)?;

    let review_messages = ["Review ", "Hash "];
    let fields = [Field {
        name: "Hash",
        value: hex_str,
    }];
    let review = MultiFieldReview::new(
        &fields,
        &review_messages,
        Some(&EYE),
        "Approve",
        Some(&CHECKMARK),
        "Reject",
        Some(&CROSS),
    );
    if review.show() {
        sign_hash(path, message)
    } else {
        Err(ErrorCode::UserCancelled)
    }
}

pub fn review_address(address: &str) -> Result<(), ErrorCode> {
    let review_messages = ["Review ", "Address "];
    let fields = [Field {
        name: "Address",
        value: address,
    }];
    let review = MultiFieldReview::new(
        &fields,
        &review_messages,
        Some(&EYE),
        "Confirm address",
        Some(&CHECKMARK),
        "Reject",
        Some(&CROSS),
    );
    if review.show() {
        Ok(())
    } else {
        Err(ErrorCode::UserCancelled)
    }
}

pub fn check_blind_signing(_tx_reviewer_inner: &mut TxReviewerInner) -> Result<(), ErrorCode> {
    if is_blind_signing_enabled() {
        return Ok(());
    }
    let page = Page::new(
        PageStyle::PictureNormal,
        ["Blind signing", "must be enabled"],
        Some(&CROSSMARK),
    );
    clear_screen();
    page.place();
    screen_update();
    let mut buttons = ButtonsState::new();

    loop {
        if let Some(ButtonEvent::BothButtonsRelease) = get_event(&mut buttons) {
            return Err(ErrorCode::BlindSigningDisabled);
        }
    }
}
