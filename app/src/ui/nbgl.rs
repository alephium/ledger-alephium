use crate::error_code::ErrorCode;
use crate::ledger_sdk_stub::nbgl_review::NbglStreamingReview;
use include_gif::include_gif;
use ledger_device_sdk::nbgl::{Field, NbglChoice, NbglGlyph, NbglReviewStatus, TransactionType};

pub static APP_ICON: NbglGlyph = NbglGlyph::from_include(include_gif!("alph_64x64.gif", NBGL));

fn new_nbgl_review(tx_type: TransactionType, blind: bool) -> NbglStreamingReview {
    let reviewer = NbglStreamingReview::new().tx_type(tx_type).glyph(&APP_ICON);
    if blind {
        reviewer.blind()
    } else {
        reviewer
    }
}

pub struct NbglReviewer {
    pub review_started: bool,
    pub display_settings: bool,
    reviewer: Option<NbglStreamingReview>,
}

impl NbglReviewer {
    pub fn new() -> NbglReviewer {
        NbglReviewer {
            review_started: false,
            display_settings: false,
            reviewer: None,
        }
    }

    pub fn reset(&mut self) {
        // Since `reset` is called when blind signing checks fails,
        // we cannot reset the `display_settings` within the reset function.
        // Instead, we will reset the `display_settings` in the `finish_review` function.
        self.review_started = false;
        self.reviewer = None;
    }

    #[inline]
    fn get_reviewer(&self) -> &NbglStreamingReview {
        assert!(self.reviewer.is_some());
        self.reviewer.as_ref().unwrap()
    }

    pub fn set_display_settings(&mut self, display_settings: bool) {
        self.display_settings = display_settings;
    }

    pub fn set_reviewer(&mut self, blind: bool) {
        assert!(self.reviewer.is_none());
        self.reviewer = Some(new_nbgl_review(TransactionType::Transaction, blind));
    }

    pub fn start_review(&mut self, message: &str) -> Result<(), ErrorCode> {
        if self.get_reviewer().start(message, "") {
            self.review_started = true;
            Ok(())
        } else {
            NbglReviewStatus::new().show(false);
            Err(ErrorCode::UserCancelled)
        }
    }

    pub fn continue_review<'a>(&self, fields: &'a [Field<'a>]) -> Result<(), ErrorCode> {
        if self.get_reviewer().continue_review(fields) {
            Ok(())
        } else {
            NbglReviewStatus::new().show(false);
            Err(ErrorCode::UserCancelled)
        }
    }

    pub fn finish_review(&mut self, message: &str) -> Result<(), ErrorCode> {
        self.display_settings = false;
        if self.get_reviewer().finish(message) {
            NbglReviewStatus::new().show(true);
            Ok(())
        } else {
            NbglReviewStatus::new().show(false);
            Err(ErrorCode::UserCancelled)
        }
    }
}

pub fn nbgl_review_hash(hash: &str) -> bool {
    let reviewer = new_nbgl_review(TransactionType::Operation, false);
    if !reviewer.start("Review Hash", "") {
        return false;
    }
    let fields = [Field {
        name: "Hash",
        value: hash,
    }];
    if !reviewer.continue_review(&fields) {
        return false;
    }
    reviewer.finish("Sign Hash")
}

pub fn nbgl_review_warning(
    message: &str,
    sub_message: &str,
    confirm_text: &str,
    cancel_text: &str,
) -> bool {
    const WARNING: NbglGlyph = NbglGlyph::from_include(include_gif!("Warning_64px.gif", NBGL));
    NbglChoice::new()
        .glyph(&WARNING)
        .show(message, sub_message, confirm_text, cancel_text)
}
