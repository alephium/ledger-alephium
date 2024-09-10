// In the ledger rust sdk, `NbglStreamingReview` uses unnecessary `&mut self`,
// which makes our code much more complicated due to Rust's borrow checker.
// Therefore, we copied it from the ledger rust sdk and changed all `&mut self` to `&self`.
// We have provided feedback to the ledger developers, and we will remove this once the SDK is updated.
// And to maintain consistency with the code in the ledger rust sdk, we ignored all clippy warnings.
#![allow(clippy::all)]

extern crate alloc;
use alloc::ffi::CString;
use alloc::vec::Vec;
use core::ffi::*;
use include_gif::include_gif;
use ledger_device_sdk::nbgl::{Field, NbglChoice, NbglGlyph, TransactionType};
use ledger_secure_sdk_sys::*;

struct CField {
    pub name: CString,
    pub value: CString,
}

/// A wrapper around the synchronous NBGL ux_sync_reviewStreaming (start, continue and finish)
/// C API binding. Used to display streamed transaction review screens.
pub struct NbglStreamingReview {
    icon: nbgl_icon_details_t,
    tx_type: TransactionType,
    blind: bool,
}

impl NbglStreamingReview {
    pub fn new() -> NbglStreamingReview {
        NbglStreamingReview {
            icon: nbgl_icon_details_t::default(),
            tx_type: TransactionType::Transaction,
            blind: false,
        }
    }

    pub fn tx_type(self, tx_type: TransactionType) -> NbglStreamingReview {
        NbglStreamingReview { tx_type, ..self }
    }

    pub fn blind(self) -> NbglStreamingReview {
        NbglStreamingReview {
            blind: true,
            ..self
        }
    }

    pub fn glyph(self, glyph: &NbglGlyph) -> NbglStreamingReview {
        NbglStreamingReview {
            icon: glyph.into(),
            ..self
        }
    }

    pub fn start(&self, title: &str, subtitle: &str) -> bool {
        unsafe {
            let title = CString::new(title).unwrap();
            let subtitle = CString::new(subtitle).unwrap();

            if self.blind {
                if !accept_blind_warning() {
                    return false;
                }
            }

            let sync_ret = ux_sync_reviewStreamingStart(
                self.tx_type.to_c_type(self.blind, false),
                &self.icon as *const nbgl_icon_details_t,
                title.as_ptr() as *const c_char,
                subtitle.as_ptr() as *const c_char,
            );

            // Return true if the user approved the transaction, false otherwise.
            match sync_ret {
                UX_SYNC_RET_APPROVED => {
                    return true;
                }
                _ => {
                    return false;
                }
            }
        }
    }

    pub fn continue_review(&self, fields: &[Field]) -> bool {
        unsafe {
            let v: Vec<CField> = fields
                .iter()
                .map(|f| CField {
                    name: CString::new(f.name).unwrap(),
                    value: CString::new(f.value).unwrap(),
                })
                .collect();

            // Fill the tag_value_array with the fields converted to nbgl_contentTagValue_t
            let mut tag_value_array: Vec<nbgl_contentTagValue_t> = Vec::new();
            for field in v.iter() {
                let val = nbgl_contentTagValue_t {
                    item: field.name.as_ptr() as *const i8,
                    value: field.value.as_ptr() as *const i8,
                    ..Default::default()
                };
                tag_value_array.push(val);
            }

            // Create the tag_value_list with the tag_value_array.
            let tag_value_list = nbgl_contentTagValueList_t {
                pairs: tag_value_array.as_ptr() as *const nbgl_contentTagValue_t,
                nbPairs: fields.len() as u8,
                ..Default::default()
            };

            let sync_ret = ux_sync_reviewStreamingContinue(
                &tag_value_list as *const nbgl_contentTagValueList_t,
            );

            // Return true if the user approved the transaction, false otherwise.
            match sync_ret {
                UX_SYNC_RET_APPROVED => {
                    return true;
                }
                _ => {
                    return false;
                }
            }
        }
    }

    pub fn finish(&self, finish_title: &str) -> bool {
        unsafe {
            let finish_title = CString::new(finish_title).unwrap();
            let sync_ret = ux_sync_reviewStreamingFinish(finish_title.as_ptr() as *const c_char);

            // Return true if the user approved the transaction, false otherwise.
            match sync_ret {
                UX_SYNC_RET_APPROVED => {
                    return true;
                }
                _ => {
                    return false;
                }
            }
        }
    }
}

/// Private helper function to display a warning screen when a transaction
/// is reviewed in "blind" mode. The user can choose to go back to safety
/// or review the risk. If the user chooses to review the risk, a second screen
/// is displayed with the option to accept the risk or reject the transaction.
/// Used in NbglReview and NbglStreamingReview.
fn accept_blind_warning() -> bool {
    const WARNING: NbglGlyph = NbglGlyph::from_include(include_gif!("Warning_64px.gif", NBGL));

    !NbglChoice::new().glyph(&WARNING)
        .show(
            "Blind signing ahead",
            "This transaction's details are not fully verifiable. If you sign it, you could lose all your assets.",
            "Back to safety",
            "Continue anyway"
        )
}
