#[cfg(not(any(target_os = "stax", target_os = "flex")))]
pub mod bagl;
#[cfg(any(target_os = "stax", target_os = "flex"))]
pub mod nbgl;

#[cfg(not(any(target_os = "stax", target_os = "flex")))]
pub use bagl::{review_address, sign_hash_ui, tx_reviewer_inner::TxReviewerInner};
#[cfg(any(target_os = "stax", target_os = "flex"))]
pub use nbgl::{review_address, sign_hash_ui, tx_reviewer_inner::TxReviewerInner};

use crate::error_code::ErrorCode;
use core::str::from_utf8;
pub mod tx_reviewer;

#[inline]
pub fn bytes_to_string(bytes: &[u8]) -> Result<&str, ErrorCode> {
    #[cfg(not(target_os = "stax"))]
    {
        match from_utf8(bytes) {
            Ok(str) => Ok(str),
            Err(_) => Err(ErrorCode::InternalError),
        }
    }

    // We encountered a strange bug on Ledger Stax where Speculos exits immediately after
    // loading the app (`syscall: os_sched_exit(0)[*] exit called (0)`), even though we haven't
    // run any tests yet. This may be a bug in Speculos, although this bug might not be related
    // to this function, the issue was resolved after changing to the following code,
    // so we implemented this workaround to address the issue.
    #[cfg(target_os = "stax")]
    {
        match from_utf8(bytes) {
            Ok(_) => Ok(from_utf8(bytes).unwrap()),
            Err(_) => Err(ErrorCode::InternalError),
        }
    }
}
