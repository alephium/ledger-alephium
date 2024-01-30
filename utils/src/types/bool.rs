use crate::buffer::Buffer;
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

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if buffer.is_empty() {
            return Ok(DecodeStage { ..*stage });
        }
        let byte = buffer.next_byte().unwrap();
        self.0 = if byte == 1 { true } else { false };
        Ok(DecodeStage::COMPLETE)
    }
}
