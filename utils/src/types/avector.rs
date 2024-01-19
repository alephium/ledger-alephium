use crate::decode::*;
use crate::buffer::Buffer;
use super::I32;

#[cfg_attr(test, derive(Debug))]
pub struct AVector<T> {
  current_item: PartialDecoder<T>,
  pub current_index: usize,
  pub total_size: I32,
}

impl <T> AVector<T> {
  pub fn get_current_item(&self) -> Option<&T> {
    if self.total_size.inner == 0 {
      None
    } else {
      Some(&self.current_item.inner)
    }
  }
}

impl <T: Default + RawDecoder> Default for AVector<T> {
  fn default() -> Self {
    AVector { current_item: new_decoder::<T>(), current_index: 0, total_size: I32::default() }
  }
}

impl <T: Default + RawDecoder> RawDecoder for AVector<T> {
  fn step_size(&self) -> usize { (self.total_size.inner as usize) + 1 }

  fn decode<'a>(&mut self, buffer: &mut Buffer<'a>, stage: &DecodeStage) -> DecodeResult<DecodeStage> {
    if stage.step == 0 {
      return self.total_size.decode(buffer, stage);
    }
    if self.step_size() == 1 { // empty avector
      return Ok(DecodeStage::COMPLETE);
    }
    if stage.step > (self.current_index + 1) {
      self.current_index += 1;
      self.current_item.reset();
    }
    self.current_item.decode_children(buffer, stage)
  }
}

#[cfg(test)]
mod tests {
  extern crate std;

  use std::vec;
  use std::vec::Vec;
  use crate::buffer::Buffer;
  use crate::decode::{new_decoder, Decoder};
  use crate::types::{U256, Hash, I32};
  use crate::types::i32::tests::random_usize;
  use crate::types::byte32::tests::gen_bytes;
  use super::AVector;

  #[test]
  fn test_decode_empty_avector() {
    let empty_avector_encoded = vec![0u8];
    let mut buffer0 = Buffer::new(&empty_avector_encoded).unwrap();
    let mut decoder0 = new_decoder::<AVector<Hash>>();
    let result0 = decoder0.decode(&mut buffer0).unwrap().unwrap();
    assert!(result0.get_current_item().is_none());

    let mut buffer1 = Buffer::new(&empty_avector_encoded).unwrap();
    let mut decoder1 = new_decoder::<AVector<U256>>();
    let result1 = decoder1.decode(&mut buffer1).unwrap().unwrap();
    assert!(result1.get_current_item().is_none());
  }

  #[test]
  fn test_decode_avector() {
    let max_size: usize = 0x1f;
    for _ in 0..10 {
      let size = random_usize(1, max_size);
      let mut hashes: Vec<Hash> = Vec::with_capacity(size);
      let mut bytes = vec![size as u8];
      for _ in 0..size {
        let hash_bytes = gen_bytes(32, 32);
        hashes.push(Hash::from_bytes(hash_bytes.as_slice().try_into().unwrap()));
        bytes.extend(&hash_bytes);
      }

      if bytes.len() <= (u8::MAX as usize) {
        let mut buffer = Buffer::new(&bytes).unwrap();
        let mut decoder = new_decoder::<AVector<Hash>>();
        let result = decoder.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(result.total_size, I32::from(size as i32));
        assert_eq!(result.get_current_item(), hashes.last());
        assert_eq!(result.current_index, size - 1);
      }

      let mut length: usize = 0;
      let mut decoder = new_decoder::<AVector<Hash>>();

      while length < bytes.len() {
        let size = if length == 0 { 33 } else { 32 };
        let mut buffer = Buffer::new(&bytes[length..(length+size)]).unwrap();
        length += size;

        let result = decoder.decode(&mut buffer).unwrap();
        if length == bytes.len() {
          assert!(result.is_some());
          assert!(decoder.stage.is_complete());
        } else {
          let index = length / 32;
          assert!(result.is_none());
          assert_eq!(decoder.stage.step, index + 1);
          assert_eq!(decoder.inner.get_current_item(), Some(&hashes[index - 1]));
        }
      }
    }
  }
}
