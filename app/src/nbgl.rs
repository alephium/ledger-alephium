extern crate alloc;
use alloc::{vec, vec::Vec};

#[cfg(any(target_os = "stax", target_os = "flex"))]
use alloc::ffi::CString;
use core::ffi::c_char;
use ledger_device_sdk::nbgl::*;
use ledger_secure_sdk_sys::*;

pub enum ReviewType {
    Transaction,
    Hash,
}

pub fn nbgl_review_fields(title: &str, subtitle: &str, fields: &TagValueList) -> bool {
    unsafe {
        let title = CString::new(title).unwrap();
        let subtitle = CString::new(subtitle).unwrap();
        let finish = CString::new("Click to continue").unwrap();
        let icon = nbgl_icon_details_t::default();
        let sync_ret = ux_sync_reviewLight(
            TYPE_TRANSACTION.into(),
            &fields.into() as *const nbgl_contentTagValueList_t,
            &icon as *const nbgl_icon_details_t,
            title.as_ptr() as *const c_char,
            subtitle.as_ptr() as *const c_char,
            finish.as_ptr() as *const c_char,
        );
        match sync_ret {
            UX_SYNC_RET_APPROVED => true,
            _ => false
        }
    }
}

pub fn nbgl_sync_review_status(tpe: ReviewType) {
    unsafe {
        let status_type = match tpe {
            ReviewType::Transaction => STATUS_TYPE_TRANSACTION_SIGNED,
            ReviewType::Hash => STATUS_TYPE_OPERATION_SIGNED, // there is no `STATUS_TYPE_HASH` in ledger sdk
        };
        let _ = ux_sync_reviewStatus(status_type);
    }
}

fn nbgl_generic_review(content: &NbglPageContent, button_str: &str) -> bool {
    unsafe {
        let (c_struct, content_type, action_callback) = content.into();
        let c_content_list: Vec<nbgl_content_t> = vec![nbgl_content_t {
            content: c_struct,
            contentActionCallback: action_callback,
            type_: content_type,
        }];

        let content_struct = nbgl_genericContents_t {
            callbackCallNeeded: false,
            __bindgen_anon_1: nbgl_genericContents_t__bindgen_ty_1 {
                contentsList: c_content_list.as_ptr() as *const nbgl_content_t,
            },
            nbContents: 1,
        };

        let button_cstring = CString::new(button_str).unwrap();

        let sync_ret = ux_sync_genericReview(
            &content_struct as *const nbgl_genericContents_t,
            button_cstring.as_ptr() as *const c_char,
        );

        // Return true if the user approved the transaction, false otherwise.
        match sync_ret {
            ledger_secure_sdk_sys::UX_SYNC_RET_APPROVED => return true,
            _ => false,
        }
    }
}

pub fn nbgl_review_warning(message: &str) -> bool {
    let content = NbglPageContent::InfoButton(InfoButton::new(message, None, "Continue", TuneIndex::TapCasual));
    nbgl_generic_review(&content, "Reject")
}

pub fn nbgl_review_info(message: &str) {
    let content = NbglPageContent::CenteredInfo(CenteredInfo::new(message, "", "", None, false, CenteredInfoStyle::NormalInfo, 0));
    let _ = nbgl_generic_review(&content, "Tap to continue");
}
