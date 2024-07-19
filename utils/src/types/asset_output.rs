use super::*;
use crate::buffer::{Buffer, Writable};
use crate::decode::*;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct AssetOutput {
    pub amount: U256,
    pub lockup_script: LockupScript,
    pub lock_time: TimeStamp,
    pub tokens: AVector<Token>,
    pub additional_data: ByteString,
}

impl Reset for AssetOutput {
    fn reset(&mut self) {
        self.amount.reset();
        self.lockup_script.reset();
        self.lock_time.reset();
        self.tokens.reset();
        self.additional_data.reset();
    }
}

impl RawDecoder for AssetOutput {
    fn step_size(&self) -> u16 {
        3 + self.tokens.step_size() + self.additional_data.step_size()
    }

    fn decode<W: Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        match stage.step {
            0 => self.amount.decode(buffer, stage),
            1 => self.lockup_script.decode(buffer, stage),
            2 => self.lock_time.decode(buffer, stage),
            step if step > 2 && step <= (2 + self.tokens.step_size()) => {
                self.tokens.decode(buffer, stage)
            }
            step if step < self.step_size() => self.additional_data.decode(buffer, stage),
            _ => Err(DecodeError::InternalError),
        }
    }
}
