extern crate alloc;
use alloc::{vec, vec::Vec};

#[cfg(any(target_os = "stax", target_os = "flex"))]
use alloc::ffi::CString;
use core::ffi::c_char;
use ledger_device_sdk::nbgl::*;
use ledger_secure_sdk_sys::*;

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

pub fn nbgl_sync_review_status() {
    unsafe {
        let _ = ux_sync_reviewStatus(STATUS_TYPE_TRANSACTION_SIGNED);
    }
}

pub fn nbgl_review_info(message: &str) {
    unsafe {
        let contents: Vec<NbglPageContent> = vec![
            NbglPageContent::CenteredInfo(CenteredInfo::new(message, "", "", None, false, CenteredInfoStyle::NormalInfo, 0))
        ];
        let c_content_list: Vec<nbgl_content_t> = contents
            .iter()
            .map(|content| {
                let (c_struct, content_type, action_callback) = content.into();
                nbgl_content_t {
                    content: c_struct,
                    contentActionCallback: action_callback,
                    type_: content_type,
                }
            })
            .collect();
        let content_struct = nbgl_genericContents_t {
            callbackCallNeeded: false,
            __bindgen_anon_1: nbgl_genericContents_t__bindgen_ty_1 {
                contentsList: c_content_list.as_ptr() as *const nbgl_content_t,
            },
            nbContents: contents.len() as u8,
        };

        let button_cstring = CString::new("Tap to continue").unwrap();
        let _ = ux_sync_genericReview(
            &content_struct as *const nbgl_genericContents_t,
            button_cstring.as_ptr() as *const c_char,
        );
    }
}
