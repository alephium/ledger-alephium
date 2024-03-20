use super::*;
use crate::buffer::{Buffer, Writable};
use crate::decode::*;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct TxInput {
    pub hint: Hint,
    pub key: Hash,
    pub unlock_script: UnlockScript,
}

impl Reset for TxInput {
    fn reset(&mut self) {
        self.hint.reset();
        self.key.reset();
        self.unlock_script.reset();
    }
}

impl RawDecoder for TxInput {
    fn step_size(&self) -> u16 {
        3
    }

    fn decode<'a, W: Writable>(
        &mut self,
        buffer: &mut Buffer<'a, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        match stage.step {
            0 => self.hint.decode(buffer, stage),
            1 => self.key.decode(buffer, stage),
            2 => self.unlock_script.decode(buffer, stage),
            _ => Err(DecodeError::InternalError),
        }
    }
}
