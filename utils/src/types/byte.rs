use crate::buffer::Buffer;
use crate::decode::*;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct Byte(pub u8);

impl RawDecoder for Byte {
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if buffer.is_empty() {
            return Ok(DecodeStage { ..*stage });
        }
        self.0 = buffer.next_byte().unwrap();
        Ok(DecodeStage::COMPLETE)
    }
}
