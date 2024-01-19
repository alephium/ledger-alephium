use crate::decode::*;
use crate::types::compact_integer::*;
use crate::buffer::Buffer;

#[cfg_attr(test, derive(Debug))]
#[derive(Default)]
pub struct I32 {
  pub inner: i32,
  first_byte: u8,
}

impl I32 {
  const SIGN_FLAG: u8 = 0x20;

  pub fn from(inner: i32) -> Self {
    I32 { inner, first_byte: 0 }
  }

  #[inline]
  pub fn get_length(&self) -> usize {
    decode_length(self.first_byte)
  }

  #[inline]
  pub fn is_fixed_size(&self) -> bool {
    is_fixed_size(self.first_byte)
  }

  fn decode_fixed_size(&mut self, buffer: &mut Buffer, length: usize, from_index: usize) -> usize {
    if from_index == 0 {
      let is_positive = self.first_byte & I32::SIGN_FLAG == 0;
      if is_positive {
        self.inner = (((self.first_byte as u32) & MASK_MODE) << ((length - 1) * 8)) as i32;
      } else {
        self.inner = (((self.first_byte as u32) | MASK_MODE_NEG) << ((length - 1) * 8)) as i32;
      }
      self.decode_i32(buffer, length, 1)
    } else {
      self.decode_i32(buffer, length, from_index)
    }
  }

  fn decode_i32(&mut self, buffer: &mut Buffer, length: usize, from_index: usize) -> usize {
    let mut index = from_index;
    while !buffer.is_empty() && index < length {
      let byte = buffer.next_byte().unwrap() as u32;
      self.inner |= ((byte & 0xff) as i32) << ((length - index - 1) * 8);
      index += 1;
    }
    index
  }
}

impl PartialEq for I32 {
  fn eq(&self, other: &Self) -> bool {
    self.inner == other.inner
  }
}

impl RawDecoder for I32 {
  fn step_size(&self) -> usize { 1 }

  fn decode<'a>(&mut self, buffer: &mut Buffer<'a>, stage: &DecodeStage) -> DecodeResult<DecodeStage> {
    if buffer.is_empty() {
      return Ok(DecodeStage { ..*stage });
    }
    if stage.index == 0 {
      self.first_byte = buffer.next_byte().unwrap();
    }
    let length = self.get_length();
    if length > 5 {
      return Err(DecodeError::InvalidSize);
    }

    let new_index = if self.is_fixed_size() {
      self.decode_fixed_size(buffer, length, stage.index)
    } else {
      let from_index = if stage.index == 0 { stage.index + 1 } else { stage.index };
      self.decode_i32(buffer, length, from_index)
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
    extern crate alloc;

    use super::*;
    use rand::Rng;
    use std::vec::Vec;

    fn hex_to_bytes(hex_string: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
      (0..hex_string.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_string[i..i + 2], 16))
        .collect()
    }

    pub fn random_usize(from: usize, to: usize) -> usize {
      let mut rng = rand::thread_rng();
      rng.gen_range(from..=to)
    }

    #[test]
    fn test_decode_i32() {
      struct TestCase<'a>(&'a str, i32);

      let arrays = [
        TestCase("00", 0),
        TestCase("01", 1),
        TestCase("02", 2),
        TestCase("3f", -1),
        TestCase("3e", -2),
        TestCase("1e", 30),
        TestCase("1f", 31),
        TestCase("4020", 32),
        TestCase("4021", 33),
        TestCase("4022", 34),
        TestCase("5ffe", 8190),
        TestCase("5fff", 8191),
        TestCase("80002000", 8192),
        TestCase("80002001", 8193),
        TestCase("80002002", 8194),
        TestCase("9ffffffe", 536870910),
        TestCase("9fffffff", 536870911),
        TestCase("c020000000", 536870912),
        TestCase("c020000001", 536870913),
        TestCase("c020000002", 536870914),
        TestCase("c07fffffff", 2147483647),
        TestCase("c080000000", -2147483648),
        TestCase("c03fffffff", 1073741823),
        TestCase("c0c0000000", -1073741824),
      ];

      for item in arrays {
        let bytes = hex_to_bytes(item.0).unwrap();
  
        {
          let mut decoder = new_decoder::<I32>();
          let mut buffer = Buffer::new(&bytes).unwrap();
          let result = decoder.decode(&mut buffer).unwrap();
          assert_eq!(result, Some(&I32::from(item.1)));
          assert!(decoder.stage.is_complete())
        }
  
        let mut length: usize = 0;
        let mut decoder = new_decoder::<I32>();
  
        while length < bytes.len() {
          let remain = bytes.len() - length;
          let size = random_usize(0, remain);
          let mut buffer = Buffer::new(&bytes[length..(length+size)]).unwrap();
          length += size;
  
          let result = decoder.decode(&mut buffer).unwrap();
          if length == bytes.len() {
            assert_eq!(result, Some(&I32::from(item.1)));
            assert!(decoder.stage.is_complete())
          } else {
            assert_eq!(result, None);
            assert_eq!(decoder.stage.index, length);
          }
        }
      }
    }
}
