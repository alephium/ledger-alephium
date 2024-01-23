use crate::decode::*;
use crate::types::compact_integer::*;
use crate::buffer::Buffer;

use super::reset;

#[cfg_attr(test, derive(Debug))]
#[derive(Default)]
pub struct I32 {
  pub inner: i32,
  first_byte: u8,
}

fn trim<'a, const NUM: usize>(dest: &'a mut [u8; NUM], is_negative: bool) -> &'a [u8] {
  let mut index = 0;
  while index < dest.len() {
    if dest[index] == b'0' {
      index += 1;
    } else {
      break;
    }
  }
  if is_negative {
    dest[index - 1] = b'-';
    &dest[(index - 1)..]
  } else {
    &dest[index..]
  }
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

  pub fn to_str<'a, const NUM: usize>(&self, output: &'a mut [u8; NUM]) -> Option<&'a [u8]> {
    reset(output);
    if output.len() < 1 { return None; }
    if self.inner == 0 {
      output[0] = b'0';
      return Some(&output[0..1]);
    }

    let num_length = if self.inner < 0 { output.len() - 1 } else { output.len() };
    let mut raw_number = self.inner;
    let mut length = 0;
    while raw_number != 0 {
      if length >= num_length { return None; }
      let index = output.len() - length - 1;
      let number = raw_number % 10;
      output[index] = b'0' + if number < 0 { (-number) as u8 } else { number as u8 };
      raw_number = raw_number / 10;
      length += 1;
    }
    Some(trim(output, self.inner < 0))
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
    use core::str::from_utf8;
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

    #[test]
    fn test_to_str() {
      let cases = [
        (0, "0"),
        (1, "1"),
        (-1, "-1"),
        (i32::MAX, "2147483647"),
        (i32::MIN, "-2147483648"),
        (111000, "111000"),
        (999999, "999999"),
      ];
      for (number, str) in cases {
        let mut output = [0; 11];
        let result = I32::from(number).to_str(&mut output);
        assert!(result.is_some());
        let expected = from_utf8(result.unwrap()).unwrap();
        assert_eq!(*str, *expected);
      }
    }
}
