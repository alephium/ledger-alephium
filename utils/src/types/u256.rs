use crate::decode::*;
use crate::types::compact_integer::*;
use crate::buffer::Buffer;

#[cfg_attr(test, derive(Debug))]
#[derive(Default)]
pub struct U256 {
  pub inner: [u64; 4],
  first_byte: u8,
}

impl PartialEq for U256 {
  fn eq(&self, other: &Self) -> bool {
    self.inner == other.inner
  }
}

impl U256 {
  #[inline]
  pub fn get_length(&self) -> usize {
    decode_length(self.first_byte)
  }

  #[inline]
  pub fn is_fixed_size(&self) -> bool {
    is_fixed_size(self.first_byte)
  }

  pub fn from(array: [u64; 4]) -> U256 {
    U256 { inner: array, first_byte: 0 }
  }

  pub fn from_u32(value: u32) -> U256 {
    Self::from_u64(value as u64)
  }

  pub fn from_u64(value: u64) -> U256 {
    U256 { inner: [0, 0, 0, value], first_byte: 0 }
  }

  fn decode_u32(&mut self, buffer: &mut Buffer, length: usize, from_index: usize) -> usize {
    let mut index = from_index;
    while !buffer.is_empty() && index < length {
      let byte = buffer.next_byte().unwrap() as u32;
      self.inner[3] |= ((byte & 0xff) as u64) << ((length - index - 1) * 8);
      index += 1;
    }
    index
  }

  fn decode_multi_bytes(&mut self, buffer: &mut Buffer, length: usize, from_index: usize) -> usize {
    let mut index = from_index;
    while !buffer.is_empty() && index < length {
      let byte = buffer.next_byte().unwrap() as u32;
      let remain = length - index - 1;
      let pos = remain - ((remain / 8) * 8);
      let u64_index = 3 - (remain / 8);
      self.inner[u64_index] |= ((byte & 0xff) as u64) << (pos * 8);
      index += 1;
    }
    index
  }

  pub fn eq(&self, value: &U256) -> bool {
    self.inner == value.inner
  }
}

impl RawDecoder for U256 {
  fn step_size(&self) -> usize { 1 }

  fn decode<'a>(&mut self, buffer: &mut Buffer<'a>, stage: &DecodeStage) -> DecodeResult<DecodeStage> {
    if buffer.is_empty() {
      return Ok(DecodeStage { ..*stage });
    }
    if stage.index == 0 {
      self.first_byte = buffer.next_byte().unwrap();
    }
    let length = self.get_length();
    let new_index = if self.is_fixed_size() {
      if stage.index == 0 {
        self.inner[3] = (((self.first_byte as u32) & MASK_MODE) << ((length - 1) * 8)) as u64;
        self.decode_u32(buffer, length, stage.index + 1)
      } else {
        self.decode_u32(buffer, length, stage.index)
      }
    } else {
      let from_index = if stage.index == 0 { stage.index + 1 } else { stage.index };
      if length == 5 {
        self.decode_u32(buffer, length, from_index)
      } else {
        self.decode_multi_bytes(buffer, length, from_index)
      }
    };
    if new_index == length {
      Ok(DecodeStage::COMPLETE)
    } else {
      Ok(DecodeStage { step: stage.step, index: new_index })
    }
  }
}

#[cfg(test)]
pub mod tests {
  extern crate std;

  use rand::Rng;
  use crate::buffer::Buffer;
  use crate::types::u256::U256 ;
  use crate::decode::*;
  use std::vec::Vec;

  pub fn hex_to_bytes(hex_string: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..hex_string.len())
      .step_by(2)
      .map(|i| u8::from_str_radix(&hex_string[i..i + 2], 16))
      .collect()
  }

  fn random_usize(from: usize, to: usize) -> usize {
    let mut rng = rand::thread_rng();
    rng.gen_range(from..=to)
  }

  pub struct TestCase<'a>(pub &'a str, pub U256);
  pub fn get_test_vector<'a>() -> [TestCase<'a>; 32] {
    let u64_max: u64 = u64::MAX;
    [
      TestCase("00", U256::from_u32(0)),
      TestCase("01", U256::from_u32(1)),
      TestCase("02", U256::from_u32(2)),
      TestCase("3e", U256::from_u32(62)),
      TestCase("3f", U256::from_u32(63)),
      TestCase("4040", U256::from_u32(64)),
      TestCase("4041", U256::from_u32(65)),
      TestCase("4042", U256::from_u32(66)),
      TestCase("7ffe", U256::from_u32(16382)),
      TestCase("7fff", U256::from_u32(16383)),
      TestCase("80004000", U256::from_u32(16384)),
      TestCase("80004001", U256::from_u32(16385)),
      TestCase("80004002", U256::from_u32(16386)),
      TestCase("bffffffe", U256::from_u32(1073741822)),
      TestCase("bfffffff", U256::from_u32(1073741823)),
      TestCase("c040000000", U256::from_u32(1073741824)),
      TestCase("c040000001", U256::from_u32(1073741825)),
      TestCase("c040000002", U256::from_u32(1073741826)),
      TestCase("c5010000000000000000", U256::from([0, 0, 1, 0])),
      TestCase("c5010000000000000001", U256::from([0, 0, 1, 1])),
      TestCase("c4ffffffffffffffff", U256::from([0, 0, 0, u64_max])),
      TestCase("cd00000000000000ff00000000000000ff00", U256::from([0, 0, 0xff00, 0xff00])),
      TestCase("cd0100000000000000000000000000000001", U256::from([0, 1, 0, 1])),
      TestCase("cd0100000000000000000000000000000000", U256::from([0, 1, 0, 0])),
      TestCase("cd0100000000000000000000000000000001", U256::from([0, 1, 0, 1])),
      TestCase("ccffffffffffffffffffffffffffffffff", U256::from([0, 0, u64_max, u64_max])),
      TestCase("d501000000000000000000000000000000000000000000000000", U256::from([1, 0, 0, 0])),
      TestCase("d501000000000000000000000000000000000000000000000001", U256::from([1, 0, 0, 1])),
      TestCase("d4ffffffffffffffffffffffffffffffffffffffffffffffff", U256::from([0, u64_max, u64_max, u64_max])),
      TestCase("dcffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", U256::from([u64_max; 4])),
      TestCase("dcfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe", U256::from([u64_max, u64_max, u64_max, u64_max - 1])),
      TestCase("dcfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd", U256::from([u64_max, u64_max, u64_max, u64_max - 2])),
    ]
  }

  #[test]
  fn test_decode_u256() {
    let arrays = get_test_vector();
    for item in arrays {
      let bytes = hex_to_bytes(item.0).unwrap();

      {
        let mut decoder = new_decoder::<U256>();
        let mut buffer = Buffer::new(&bytes).unwrap();
        let result = decoder.decode(&mut buffer).unwrap();
        assert_eq!(result, Some(&item.1));
        assert!(decoder.stage.is_complete())
      }

      let mut length: usize = 0;
      let mut decoder = new_decoder::<U256>();

      while length < bytes.len() {
        let remain = bytes.len() - length;
        let size = random_usize(0, remain);
        let mut buffer = Buffer::new(&bytes[length..(length+size)]).unwrap();
        length += size;

        let result = decoder.decode(&mut buffer).unwrap();
        if length == bytes.len() {
          assert_eq!(result, Some(&item.1));
          assert!(decoder.stage.is_complete())
        } else {
          assert_eq!(result, None);
          assert_eq!(decoder.stage.index, length);
        }
      }
    }
  }
}
