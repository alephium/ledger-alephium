use crate::buffer::Buffer;

#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum DecodeError {
  InvalidSize,
  InvalidData,
  InternalError,
  NotSupported,
}

pub type DecodeResult<T> = Result<T, DecodeError>;

#[cfg_attr(test, derive(Debug))]
#[derive(Default, PartialEq)]
pub struct DecodeStage {
  pub step: usize,
  pub index: usize,
}

impl DecodeStage {
  pub const COMPLETE: DecodeStage = DecodeStage { step: usize::MAX, index: usize::MAX };

  pub fn is_complete(&self) -> bool {
    *self == Self::COMPLETE
  }

  pub fn next_step(&self) -> DecodeStage {
    DecodeStage { step: self.step + 1, index: 0 }
  }
}

pub trait RawDecoder: Sized {
  fn step_size(&self) -> usize;

  fn decode<'a>(&mut self, buffer: &mut Buffer<'a>, stage: &DecodeStage) -> DecodeResult<DecodeStage>;
}

pub trait Decoder<T>: Sized {
  fn decode<'a>(&mut self, buffer: &mut Buffer<'a>) -> DecodeResult<Option<&T>>;
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct PartialDecoder<T> {
  pub inner: T,
  pub stage: DecodeStage,
}

impl <T: Default> Default for PartialDecoder<T> {
  fn default() -> Self {
    PartialDecoder { inner: T::default(), stage: DecodeStage::default() }
  }
}

impl <T: Default + RawDecoder> Decoder<T> for PartialDecoder<T> {
  fn decode<'a>(&mut self, buffer: &mut Buffer<'a>) -> DecodeResult<Option<&T>> {
    if self.stage.is_complete() {
      return Ok(Some(&self.inner))
    }
    if self.stage.step >= self.inner.step_size() {
      return Err(DecodeError::InternalError);
    }
    match self.inner.decode(buffer, &self.stage) {
      Ok(stage) => {
        let stage = if stage.is_complete() { self.stage.next_step() } else { stage };
        self.stage = if stage.step == self.inner.step_size() { DecodeStage::COMPLETE } else { stage };
        if self.stage.is_complete() {
          Ok(Some(&self.inner))
        } else if !buffer.is_empty() {
          self.decode(buffer)
        } else {
          Ok(None)
        }
      },
      Err(err) => Err(err),
    }
  }
}

impl <T: Default + RawDecoder> PartialDecoder<T> {
  pub fn decode_children<'a>(&mut self, buffer: &mut Buffer<'a>, parent_stage: &DecodeStage) -> DecodeResult<DecodeStage> {
    self.decode(buffer).map(|result| {
      if result.is_some() {
        DecodeStage::COMPLETE
      } else {
        DecodeStage { ..*parent_stage }
      }
    })
  }
}

pub fn new_decoder<T: Default + RawDecoder>() -> PartialDecoder<T> {
  PartialDecoder::<T>::default()
}
