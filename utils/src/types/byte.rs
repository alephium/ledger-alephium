use crate::buffer::{Buffer, Writable};
use crate::decode::*;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct Byte(pub u8);

impl Reset for Byte {
    fn reset(&mut self) {
        self.0 = 0;
    }
}

impl RawDecoder for Byte {
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
        self.0 = buffer.consume_byte().unwrap();
        Ok(DecodeStage::COMPLETE)
    }
}
