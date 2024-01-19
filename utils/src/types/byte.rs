use crate::decode::*;
use crate::buffer::Buffer;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct Byte(pub u8);

impl RawDecoder for Byte {
  fn step_size(&self) -> usize { 1 }

  fn decode<'a>(&mut self, buffer: &mut Buffer<'a>, stage: &DecodeStage) -> DecodeResult<DecodeStage> {
    if buffer.is_empty() {
      return Ok(DecodeStage { ..*stage });
    }
    self.0 = buffer.next_byte().unwrap();
    Ok(DecodeStage::COMPLETE)
  }
}
