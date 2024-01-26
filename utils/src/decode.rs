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
    pub step: u16,
    pub index: u16,
}

impl DecodeStage {
    pub const COMPLETE: DecodeStage = DecodeStage {
        step: u16::MAX,
        index: u16::MAX,
    };

    pub fn is_complete(&self) -> bool {
        *self == Self::COMPLETE
    }

    pub fn next_step(&self) -> DecodeStage {
        DecodeStage {
            step: self.step + 1,
            index: 0,
        }
    }
}

pub trait RawDecoder: Sized {
    fn step_size(&self) -> u16;

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage>;
}

pub trait Decoder<T>: Sized {
    fn decode<'a>(&mut self, buffer: &mut Buffer<'a>) -> DecodeResult<Option<&T>>;
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct PartialDecoder<T> {
    pub inner: T,
    pub stage: DecodeStage,
}

impl<T: Default> PartialDecoder<T> {
    pub fn reset(&mut self) {
        self.inner = T::default();
        self.stage.index = 0;
        self.stage.step = 0;
    }
}

impl<T: Default> Default for PartialDecoder<T> {
    fn default() -> Self {
        PartialDecoder {
            inner: T::default(),
            stage: DecodeStage::default(),
        }
    }
}

impl<T: RawDecoder> PartialDecoder<T> {
    pub fn try_decode_one_step<'a>(&mut self, buffer: &mut Buffer<'a>) -> DecodeResult<bool> {
        if buffer.is_empty() {
            return Ok(false);
        }
        if self.stage.step >= self.inner.step_size() {
            return Err(DecodeError::InternalError);
        }
        match self.inner.decode(buffer, &self.stage) {
            Ok(stage) => {
                let result = stage.is_complete();
                let stage = if stage.is_complete() {
                    self.stage.next_step()
                } else {
                    stage
                };
                self.stage = if stage.step == self.inner.step_size() {
                    DecodeStage::COMPLETE
                } else {
                    stage
                };
                Ok(result)
            }
            Err(err) => Err(err),
        }
    }

    pub fn decode_children<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        parent_stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        self.decode(buffer).map(|result| {
            if result.is_some() {
                DecodeStage::COMPLETE
            } else {
                DecodeStage { ..*parent_stage }
            }
        })
    }
}

impl<T: RawDecoder> Decoder<T> for PartialDecoder<T> {
    fn decode<'a>(&mut self, buffer: &mut Buffer<'a>) -> DecodeResult<Option<&T>> {
        match self.try_decode_one_step(buffer) {
            Ok(true) => {
                if self.stage.is_complete() {
                    Ok(Some(&self.inner))
                } else {
                    self.decode(buffer)
                }
            }
            Ok(false) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

impl<T: Default + RawDecoder> RawDecoder for Option<T> {
    fn step_size(&self) -> u16 {
        match self {
            None => 1,
            Some(v) => v.step_size(),
        }
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if buffer.is_empty() {
            return Ok(DecodeStage { ..*stage });
        }

        if self.is_none() {
            let byte = buffer.next_byte().unwrap();
            if byte == 0 {
                return Ok(DecodeStage::COMPLETE);
            } else if byte == 1 {
                *self = Some(T::default());
            } else {
                return Err(DecodeError::InvalidData);
            }
        }

        match self {
            Some(v) => v.decode(buffer, stage),
            None => Err(DecodeError::InternalError),
        }
    }
}

impl<T1: RawDecoder, T2: RawDecoder> RawDecoder for (T1, T2) {
    fn step_size(&self) -> u16 {
        self.0.step_size() + self.1.step_size()
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        match stage.step {
            step if step < self.0.step_size() => self.0.decode(buffer, stage),
            step if step < self.step_size() => self.1.decode(buffer, stage),
            _ => Err(DecodeError::InternalError),
        }
    }
}

pub fn new_decoder<T: Default + RawDecoder>() -> PartialDecoder<T> {
    PartialDecoder::<T>::default()
}
