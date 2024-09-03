// The purpose of this file is to address the issue of the ledger rust sdk not supporting
// navigation to the settings page.
// To maintain consistency with the ledger rust sdk code, we have ignored clipply warnings here.
#![allow(clippy::all)]

use crate::settings::{SETTINGS_DATA, SETTINGS_SIZE};
use const_zero::const_zero;

extern crate alloc;
use alloc::ffi::CString;
use alloc::vec::Vec;
use core::ffi::*;
use core::mem::transmute;
use include_gif::include_gif;
use ledger_device_sdk::io::{ApduHeader, Comm, Event, Reply};
use ledger_device_sdk::nbgl::{NbglGlyph, TuneIndex};
use ledger_device_sdk::nvm::{AtomicStorage, SingleStorage};
use ledger_secure_sdk_sys::*;

static mut NVM_REF: Option<&mut AtomicStorage<[u8; SETTINGS_SIZE]>> = None;
static mut SWITCH_ARRAY: [nbgl_contentSwitch_t; SETTINGS_SIZE] =
    [unsafe { const_zero!(nbgl_contentSwitch_t) }; SETTINGS_SIZE];
static mut SETTINGS_UPDATED: bool = false;

/// Information fields name to display in the dedicated
/// page of the home screen.
const INFO_FIELDS: [*const c_char; 2] = [
    "Version\0".as_ptr() as *const c_char,
    "Developer\0".as_ptr() as *const c_char,
];

pub fn nbgl_display<'a, T: TryFrom<ApduHeader>>(
    comm: &mut Comm,
    settings_strings: &[[&'a str; 2]],
    page: u8,
) -> Event<T>
where
    Reply: From<<T as TryFrom<ApduHeader>>::Error>,
{
    let mut info_contents: Vec<CString> = Vec::new();
    info_contents.push(CString::new("Alephium").unwrap());
    info_contents.push(CString::new(env!("CARGO_PKG_VERSION")).unwrap());
    info_contents.push(CString::new(env!("CARGO_PKG_AUTHORS")).unwrap());

    unsafe {
        NVM_REF = Some(transmute(SETTINGS_DATA.get_mut()));
    }

    let nb_settings = settings_strings.len() as u8;
    let setting_contents: Vec<[CString; 2]> = settings_strings
        .iter()
        .map(|s| [CString::new(s[0]).unwrap(), CString::new(s[1]).unwrap()])
        .collect();

    const APP_ICON: NbglGlyph = NbglGlyph::from_include(include_gif!("alph_64x64.gif", NBGL));
    unsafe {
        let mut page_index = page;
        'outer: loop {
            let info_contents: Vec<*const c_char> =
                info_contents.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();

            let info_list: nbgl_contentInfoList_t = nbgl_contentInfoList_t {
                infoTypes: INFO_FIELDS.as_ptr() as *const *const c_char,
                infoContents: info_contents[1..].as_ptr() as *const *const c_char,
                nbInfos: INFO_FIELDS.len() as u8,
            };

            let icon: nbgl_icon_details_t = (&APP_ICON).into();

            for (i, setting) in setting_contents.iter().enumerate() {
                SWITCH_ARRAY[i].text = setting[0].as_ptr();
                SWITCH_ARRAY[i].subText = setting[1].as_ptr();
                SWITCH_ARRAY[i].initState = NVM_REF.as_mut().unwrap().get_ref()[i] as nbgl_state_t;
                SWITCH_ARRAY[i].token = (FIRST_USER_TOKEN + i as u32) as u8;
                SWITCH_ARRAY[i].tuneId = TuneIndex::TapCasual as u8;
            }

            let content: nbgl_content_t = nbgl_content_t {
                content: nbgl_content_u {
                    switchesList: nbgl_pageSwitchesList_s {
                        switches: &SWITCH_ARRAY as *const nbgl_contentSwitch_t,
                        nbSwitches: nb_settings,
                    },
                },
                contentActionCallback: Some(settings_callback),
                type_: SWITCHES_LIST,
            };

            let generic_contents: nbgl_genericContents_t = nbgl_genericContents_t {
                callbackCallNeeded: false,
                __bindgen_anon_1: nbgl_genericContents_t__bindgen_ty_1 {
                    contentsList: &content as *const nbgl_content_t,
                },
                nbContents: if nb_settings > 0 { 1 } else { 0 },
            };

            nbgl_useCaseHomeAndSettings(
                info_contents[0],
                &icon as *const nbgl_icon_details_t,
                core::ptr::null(),
                page_index,
                &generic_contents as *const nbgl_genericContents_t,
                &info_list as *const nbgl_contentInfoList_t,
                core::ptr::null(),
                Some(app_exit),
            );
            loop {
                match comm.next_event() {
                    Event::Command(t) => return Event::Command(t),
                    _ => {
                        if SETTINGS_UPDATED {
                            SETTINGS_UPDATED = false;
                            page_index = 0; // display the settings page
                            continue 'outer;
                        }
                    }
                }
            }
        }
    }
}

/// Callback triggered by the NBGL API when a setting switch is toggled.
unsafe extern "C" fn settings_callback(token: c_int, _index: u8, _page: c_int) {
    let idx = token - FIRST_USER_TOKEN as i32;
    if idx < 0 || idx >= SETTINGS_SIZE as i32 {
        panic!("Invalid token.");
    }

    if let Some(data) = NVM_REF.as_mut() {
        let setting_idx: usize = idx as usize;
        let mut switch_values: [u8; SETTINGS_SIZE] = data.get_ref().clone();
        switch_values[setting_idx] = !switch_values[setting_idx];
        data.update(&switch_values);
        SWITCH_ARRAY[setting_idx].initState = switch_values[setting_idx] as nbgl_state_t;
        SETTINGS_UPDATED = true;
    }
}

unsafe extern "C" fn app_exit() {
    os_sched_exit(1);
}
