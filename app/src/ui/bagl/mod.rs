pub mod home;
pub mod tx_reviewer_inner;

use crate::{error_code::ErrorCode, public_key::sign_hash};
use core::str::from_utf8;
use ledger_device_sdk::ui::bitmaps::{CHECKMARK, CROSS, EYE};
use ledger_device_sdk::ui::gadgets::{Field, MultiFieldReview};

pub fn sign_hash_ui(path: &[u32], message: &[u8]) -> Result<([u8; 72], u32, u32), ErrorCode> {
    let hex: [u8; 64] = utils::to_hex(message).ok_or(ErrorCode::BadLen)?;
    let hex_str = from_utf8(&hex).map_err(|_| ErrorCode::InternalError)?;

    let review_messages = ["Review", "Hash"];
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
    let review_messages = ["Review", "Address"];
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
