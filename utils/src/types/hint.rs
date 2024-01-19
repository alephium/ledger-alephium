use crate::decode::*;
use crate::buffer::Buffer;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct Hint([u8; 4]);

impl Hint {
  const ENCODED_LENGTH: usize = 4;

  pub fn from_bytes(bytes: [u8; 4]) -> Self {
    Hint(bytes)
  }
}

impl RawDecoder for Hint {
  fn step_size(&self) -> usize { 1}

  fn decode<'a>(&mut self, buffer: &mut Buffer<'a>, stage: &DecodeStage) -> DecodeResult<DecodeStage> {
    let remain = Hint::ENCODED_LENGTH - stage.index;
    let mut idx: usize = 0;
    while !buffer.is_empty() && idx < remain {
      self.0[stage.index + idx] = buffer.next_byte().unwrap();
      idx += 1;
    }
    let new_index = stage.index + idx;
    if new_index == Hint::ENCODED_LENGTH {
      Ok(DecodeStage::COMPLETE)
    } else {
      Ok(DecodeStage { step: stage.step, index: new_index })
    }
  }
}

#[cfg(test)]
mod tests {
  extern crate std;

  use crate::buffer::Buffer;
  use crate::decode::Decoder;
  use std::vec::Vec;
  use std::vec;
  use rand::Rng;
  use super::{Hint, new_decoder};

  fn gen_bytes(min_length: usize, max_length: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let length = rng.gen_range(min_length..=max_length);
    let mut random_bytes = vec![0u8; length];
    rng.fill(&mut random_bytes[..]);
    random_bytes
  }

  #[test]
  fn test_decode_hint() {
    let mut bytes = vec![0u8; 0];
    let mut decoder = new_decoder::<Hint>();

    while bytes.len() < Hint::ENCODED_LENGTH {
      let data = gen_bytes(0, Hint::ENCODED_LENGTH * 2);
      let mut buffer = Buffer::new(data.as_slice()).unwrap();
      bytes.extend(&data);

      let result = decoder.decode(&mut buffer);
      if bytes.len() < Hint::ENCODED_LENGTH {
        assert_eq!(result, Ok(None));
      } else {
        let array: [u8; 4] = bytes.as_slice()[0..Hint::ENCODED_LENGTH].try_into().unwrap();
        assert_eq!(result, Ok(Some(&Hint::from_bytes(array))));
      }
    }
  }
}