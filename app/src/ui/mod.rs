#[cfg(not(any(target_os = "stax", target_os = "flex")))]
pub mod bagl;
#[cfg(any(target_os = "stax", target_os = "flex"))]
pub mod nbgl;

#[cfg(not(any(target_os = "stax", target_os = "flex")))]
pub use bagl::{
    check_blind_signing, review_address, sign_hash_ui, tx_reviewer_inner::TxReviewerInner,
};
#[cfg(any(target_os = "stax", target_os = "flex"))]
pub use nbgl::{
    check_blind_signing, review_address, sign_hash_ui, tx_reviewer_inner::TxReviewerInner,
};

use crate::error_code::ErrorCode;
use core::str::from_utf8;
pub mod tx_reviewer;

#[inline]
pub fn bytes_to_string(bytes: &[u8]) -> Result<&str, ErrorCode> {
    match from_utf8(bytes) {
        Ok(str) => Ok(str),
        Err(_) => Err(ErrorCode::InternalError),
    }
}
