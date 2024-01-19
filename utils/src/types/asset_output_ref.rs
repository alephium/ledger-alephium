use crate::decode::*;
use crate::buffer::Buffer;
use super::*;

#[cfg_attr(test, derive(Debug))]
#[derive(Default)]
pub struct AssetOutputRef {
  hint: Hint,
  key: Hash,
}

impl RawDecoder for AssetOutputRef {
  fn step_size(&self) -> usize { 2 }

  fn decode<'a>(&mut self, buffer: &mut Buffer<'a>, stage: &DecodeStage) -> DecodeResult<DecodeStage> {
    match stage.step {
      0 => self.hint.decode(buffer, stage),
      1 => self.key.decode(buffer, stage),
      _ => Err(DecodeError::InternalError),
    }
  }
}
