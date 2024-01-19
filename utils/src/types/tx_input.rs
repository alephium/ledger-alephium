use crate::decode::*;
use crate::buffer::Buffer;
use super::*;

#[cfg_attr(test, derive(Debug))]
#[derive(Default)]
pub struct TxInput {
  pub output_ref: PartialDecoder<AssetOutputRef>,
  pub unlock_script: UnlockScript,
}

impl RawDecoder for TxInput {
  fn step_size(&self) -> usize { 2 }

  fn decode<'a>(&mut self, buffer: &mut Buffer<'a>, stage: &DecodeStage) -> DecodeResult<DecodeStage> {
    match stage.step {
      0 => self.output_ref.decode_children(buffer, stage),
      1 => self.unlock_script.decode(buffer, stage),
      _ => Err(DecodeError::InternalError),
    }
  }
}
