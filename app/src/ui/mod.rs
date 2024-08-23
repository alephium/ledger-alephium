use core::str::from_utf8;
use ledger_device_sdk::ecc::ECPublicKey;

use crate::{
    error_code::ErrorCode,
    public_key::{sign_hash, Address},
};

#[cfg(not(any(target_os = "stax", target_os = "flex")))]
pub mod display;
#[cfg(any(target_os = "stax", target_os = "flex"))]
pub mod nbgl;

pub mod tx_reviewer;

pub fn sign_hash_ui(path: &[u32], message: &[u8]) -> Result<([u8; 72], u32, u32), ErrorCode> {
    let hex: [u8; 64] = utils::to_hex(message).ok_or(ErrorCode::BadLen)?;
    let hex_str = from_utf8(&hex).map_err(|_| ErrorCode::InternalError)?;

    #[cfg(not(any(target_os = "stax", target_os = "flex")))]
    {
        use crate::ledger_sdk_stub::multi_field_review::{Field, MultiFieldReview};
        use ledger_device_sdk::ui::bitmaps::{CHECKMARK, CROSS, EYE};

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

    #[cfg(any(target_os = "stax", target_os = "flex"))]
    {
        use crate::ui::nbgl::{nbgl_review_fields, nbgl_sync_review_status, ReviewType};
        use ledger_device_sdk::nbgl::{Field, TagValueList};

        let fields = [Field {
            name: "Hash",
            value: hex_str,
        }];
        let values = TagValueList::new(&fields, 0, false, false);
        let approved = nbgl_review_fields("Review", "Hash", &values);
        if approved {
            nbgl_sync_review_status(ReviewType::Hash);
            sign_hash(path, message)
        } else {
            Err(ErrorCode::UserCancelled)
        }
    }
}

pub fn review_address(pub_key: &ECPublicKey<65, 'W'>) -> Result<(), ErrorCode> {
    let address = Address::from_pub_key(pub_key)?;
    let address_str = address.get_address_str()?;

    #[cfg(not(any(target_os = "stax", target_os = "flex")))]
    {
        use crate::ledger_sdk_stub::multi_field_review::{Field, MultiFieldReview};
        use ledger_device_sdk::ui::bitmaps::{CHECKMARK, CROSS, EYE};

        let review_messages = ["Review ", "Address "];
        let fields = [Field {
            name: "Address",
            value: address_str,
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

    #[cfg(any(target_os = "stax", target_os = "flex"))]
    {
        use include_gif::include_gif;
        use ledger_device_sdk::nbgl::{NbglAddressReview, NbglGlyph};

        const APP_ICON: NbglGlyph = NbglGlyph::from_include(include_gif!("alph_64x64.gif", NBGL));
        let result = NbglAddressReview::new()
            .glyph(&APP_ICON)
            .verify_str("Verify Alephium address")
            .show(address_str);
        if result {
            Ok(())
        } else {
            Err(ErrorCode::UserCancelled)
        }
    }
}
