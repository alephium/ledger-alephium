use crate::buffer::{Buffer, Writable};
use crate::decode::*;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct Bool(bool);

impl Reset for Bool {
    fn reset(&mut self) {
        self.0 = false;
    }
}

impl RawDecoder for Bool {
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<W: Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if buffer.is_empty() {
            return Ok(DecodeStage { ..*stage });
        }
        let byte = buffer.consume_byte().unwrap();
        self.0 = byte == 1;
        Ok(DecodeStage::COMPLETE)
    }
}
