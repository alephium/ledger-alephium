pub mod tx_reviewer_inner;

use crate::{
    error_code::ErrorCode, ledger_sdk_stub::nbgl_review::NbglStreamingReview, public_key::sign_hash,
};
use core::str::from_utf8;
use include_gif::include_gif;
use ledger_device_sdk::nbgl::{
    Field, NbglAddressReview, NbglChoice, NbglGlyph, NbglReviewStatus, TransactionType,
};

pub static APP_ICON: NbglGlyph = NbglGlyph::from_include(include_gif!("alph_64x64.gif", NBGL));

fn new_nbgl_review(tx_type: TransactionType, blind: bool) -> NbglStreamingReview {
    let reviewer = NbglStreamingReview::new().tx_type(tx_type).glyph(&APP_ICON);
    if blind {
        reviewer.blind()
    } else {
        reviewer
    }
}

fn nbgl_review_hash(hash: &str) -> bool {
    let reviewer = new_nbgl_review(TransactionType::Operation, false);
    if !reviewer.start("Review Hash", "") {
        return false;
    }
    let fields = [Field {
        name: "Hash",
        value: hash,
    }];
    if !reviewer.continue_review(&fields) {
        return false;
    }
    reviewer.finish("Sign Hash")
}

pub fn nbgl_review_warning(
    message: &str,
    sub_message: &str,
    confirm_text: &str,
    cancel_text: &str,
) -> bool {
    const WARNING: NbglGlyph = NbglGlyph::from_include(include_gif!("Warning_64px.gif", NBGL));
    NbglChoice::new()
        .glyph(&WARNING)
        .show(message, sub_message, confirm_text, cancel_text)
}

pub fn sign_hash_ui(path: &[u32], message: &[u8]) -> Result<([u8; 72], u32, u32), ErrorCode> {
    let hex: [u8; 64] = utils::to_hex(message).ok_or(ErrorCode::BadLen)?;
    match from_utf8(&hex) {
        Ok(hex_str) => {
            if nbgl_review_hash(hex_str) {
                NbglReviewStatus::new().show(true);
                sign_hash(path, message)
            } else {
                NbglReviewStatus::new().show(false);
                Err(ErrorCode::UserCancelled)
            }
        }
        Err(_) => Err(ErrorCode::InternalError),
    }
}

pub fn review_address(address: &str) -> Result<(), ErrorCode> {
    let result = NbglAddressReview::new()
        .glyph(&APP_ICON)
        .verify_str("Verify Alephium address")
        .show(address);
    if result {
        Ok(())
    } else {
        Err(ErrorCode::UserCancelled)
    }
}
