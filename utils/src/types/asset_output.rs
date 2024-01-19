use crate::buffer::Buffer;
use crate::decode::*;
use super::*;

#[cfg_attr(test, derive(Debug))]
#[derive(Default)]
pub struct AssetOutput {
  pub amount: U256,
  pub lockup_script: LockupScript,
  pub lock_time: TimeStamp,
  pub tokens: PartialDecoder<AVector<Token>>,
  pub additional_data: PartialDecoder<ByteString>,
}

impl RawDecoder for AssetOutput {
  fn step_size(&self) -> usize { 5 }

  fn decode<'a>(&mut self, buffer: &mut Buffer<'a>, stage: &DecodeStage) -> DecodeResult<DecodeStage> {
    match stage.step {
      0 => self.amount.decode(buffer, stage),
      1 => self.lockup_script.decode(buffer, stage),
      2 => self.lock_time.decode(buffer, stage),
      3 => self.tokens.decode_children(buffer, stage),
      4 => self.additional_data.decode_children(buffer, stage),
      _ => Err(DecodeError::InternalError),
    }
  }
}
