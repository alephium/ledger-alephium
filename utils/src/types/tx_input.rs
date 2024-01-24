use super::*;
use crate::buffer::Buffer;
use crate::decode::*;

#[cfg_attr(test, derive(Debug))]
#[derive(Default)]
pub struct TxInput {
    pub hint: Hint,
    pub key: Hash,
    pub unlock_script: UnlockScript,
}

impl RawDecoder for TxInput {
    fn step_size(&self) -> usize {
        3
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
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
