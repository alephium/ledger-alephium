use super::*;
use crate::buffer::{Buffer, Writable};
use crate::decode::*;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct Script(AVector<Method>);

impl Reset for Script {
    fn reset(&mut self) {
        self.0.reset();
    }
}

impl RawDecoder for Script {
    fn step_size(&self) -> u16 {
        self.0.step_size()
    }

    fn decode<'a, W: Writable>(
        &mut self,
        buffer: &mut Buffer<'a, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        match stage.step {
            step if step < self.step_size() => self.0.decode(buffer, stage),
            _ => Err(DecodeError::InternalError),
        }
    }
}
