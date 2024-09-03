use crate::error_code::ErrorCode;
use crate::ledger_sdk_stub::multi_field_review::{Field, MultiFieldReview};
use ledger_device_sdk::ui::bitmaps::{CHECKMARK, CROSS, EYE, WARNING};

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
        let review = MultiFieldReview::new(
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
    pub fn review_self_transfer(&self, fee_field: &Field) -> Result<(), ErrorCode> {
        let fields = &[Field {
            name: fee_field.name,
            value: fee_field.value,
        }];
        let review = if self.is_tx_execute_script {
            MultiFieldReview::new(
                fields,
                &["Blind Signing"],
                Some(&WARNING),
                "Sign transaction",
                Some(&CHECKMARK),
                "Reject",
                Some(&CROSS),
            )
        } else {
            MultiFieldReview::new(
                fields,
                &["Confirm ", "Self-transfer"],
                Some(&EYE),
                "Sign transaction",
                Some(&CHECKMARK),
                "Reject",
                Some(&CROSS),
            )
        };
        if review.show() {
            Ok(())
        } else {
            Err(ErrorCode::UserCancelled)
        }
    }

    // Review the warning for external inputs, i.e. inputs that are not from the device address
    pub fn warning_external_inputs(&self) -> Result<(), ErrorCode> {
        let review_messages = ["There are ", "external inputs"];
        let review = MultiFieldReview::new(
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
        let review = MultiFieldReview::new(
            fields,
            &[],
            None,
            "Sign transaction",
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
}
