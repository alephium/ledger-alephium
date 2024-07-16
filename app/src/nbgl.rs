extern crate alloc;

#[cfg(any(target_os = "stax", target_os = "flex"))]
use alloc::ffi::CString;
use core::ffi::c_char;
use ledger_device_sdk::nbgl::TagValueList;
use ledger_secure_sdk_sys::{
    TYPE_TRANSACTION, nbgl_icon_details_t,
    nbgl_contentTagValueList_t, UX_SYNC_RET_APPROVED, ux_sync_reviewStatus,
    STATUS_TYPE_TRANSACTION_SIGNED, ux_sync_reviewLight
};

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
