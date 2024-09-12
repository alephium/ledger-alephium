use crate::error_code::ErrorCode;
use crate::ledger_sdk_stub::multi_field_review::{Field, MultiFieldReview};
use crate::settings::is_blind_signing_enabled;
use ledger_device_sdk::{
    buttons::{ButtonEvent, ButtonsState},
    ui::bitmaps::{Glyph, CHECKMARK, CROSS, CROSSMARK, EYE, WARNING},
    ui::gadgets::{clear_screen, get_event, Page, PageStyle},
    ui::screen_util::screen_update,
};

// Different Ledger devices use different UI libraries, so we've introduced the
// `TxReviewInner` to facilitate the display of tx details across different devices.
// The `TxReviewInner` here is for Ledger Nanosp/Nanox.
pub struct TxReviewerInner {
    is_tx_execute_script: bool,
}

impl TxReviewerInner {
    pub fn new() -> TxReviewerInner {
        TxReviewerInner {
            is_tx_execute_script: false,
        }
    }

    // Start review tx details
    #[inline]
    pub fn start_review(&self) -> Result<(), ErrorCode> {
        Ok(())
    }

    pub fn review_fields<'a>(
        &self,
        fields: &'a [Field<'a>],
        review_message: &str,
    ) -> Result<(), ErrorCode> {
        let review_messages = ["Review ", review_message];
        let review = MultiFieldReview::simple(
            fields,
            &review_messages,
            Some(&EYE),
            "Continue",
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

    // Review transfer that sends to self
    pub fn review_self_transfer(&self, fee_field: Field) -> Result<(), ErrorCode> {
        let fields = &[fee_field];
        if self.is_tx_execute_script {
            self.finish_review_inner(fields, &["Blind Signing"], Some(&WARNING))
        } else {
            self.finish_review_inner(fields, &["Confirm ", "Self-transfer"], Some(&EYE))
        }
    }

    // Review the warning for external inputs, i.e. inputs that are not from the device address
    pub fn warning_external_inputs(&self) -> Result<(), ErrorCode> {
        let review_messages = ["There are ", "external inputs"];
        let review = MultiFieldReview::simple(
            &[],
            &review_messages,
            Some(&WARNING),
            "Continue",
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

    pub fn finish_review<'a>(&self, fields: &'a [Field<'a>]) -> Result<(), ErrorCode> {
        self.finish_review_inner(fields, &[], None)
    }

    fn finish_review_inner<'a>(
        &self,
        fields: &'a [Field<'a>],
        review_message: &'a [&'a str],
        review_glyph: Option<&'a Glyph<'a>>,
    ) -> Result<(), ErrorCode> {
        let validation_message = if !self.is_tx_execute_script {
            ["Sign transaction", ""]
        } else {
            ["Accept risk and", "sign transaction?"]
        };

        let review = MultiFieldReview::new(
            fields,
            review_message,
            review_glyph,
            validation_message,
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

    #[inline]
    pub fn set_tx_execute_script(&mut self, is_tx_execute_script: bool) {
        self.is_tx_execute_script = is_tx_execute_script;
    }

    #[inline]
    pub fn reset(&mut self) {
        self.is_tx_execute_script = false;
    }

    #[inline]
    pub fn output_index_as_field(&self) -> bool {
        false
    }

    pub fn check_blind_signing(&self) -> Result<(), ErrorCode> {
        if is_blind_signing_enabled() {
            return Ok(());
        }
        let page = Page::new(
            PageStyle::PictureNormal,
            ["Blind signing", "must be enabled"],
            Some(&CROSSMARK),
        );
        clear_screen();
        page.place();
        screen_update();
        let mut buttons = ButtonsState::new();

        loop {
            if let Some(ButtonEvent::BothButtonsRelease) = get_event(&mut buttons) {
                return Err(ErrorCode::BlindSigningDisabled);
            }
        }
    }
}
