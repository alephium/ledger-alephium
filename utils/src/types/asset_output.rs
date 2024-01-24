use super::*;
use crate::buffer::Buffer;
use crate::decode::*;

#[cfg_attr(test, derive(Debug))]
#[derive(Default)]
pub struct AssetOutput {
    pub amount: U256,
    pub lockup_script: LockupScript,
    pub lock_time: TimeStamp,
    pub tokens: AVector<Token>,
    pub additional_data: ByteString, // TODO: improve decode byte string
}

impl RawDecoder for AssetOutput {
    fn step_size(&self) -> usize {
        3 + self.tokens.step_size() + self.additional_data.step_size()
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        match stage.step {
            0 => self.amount.decode(buffer, stage),
            1 => self.lockup_script.decode(buffer, stage),
            2 => self.lock_time.decode(buffer, stage),
            step if step > 2 && step <= (2 + self.tokens.step_size()) => {
                self.tokens.decode(buffer, stage)
            }
            step if step <= self.step_size() => self.additional_data.decode(buffer, stage),
            _ => Err(DecodeError::InternalError),
        }
    }
}
