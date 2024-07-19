use crate::buffer::{Buffer, Writable};
use crate::decode::*;
use crate::types::compact_integer::*;

use super::reset;

#[cfg_attr(test, derive(Debug))]
#[derive(Default)]
pub struct I32 {
    pub inner: i32,
    first_byte: u8,
}

impl Reset for I32 {
    fn reset(&mut self) {
        self.inner = 0;
        self.first_byte = 0;
    }
}

fn trim(dest: &mut [u8], is_negative: bool) -> &[u8] {
    let mut index = 0;
    while index < dest.len() {
        if dest[index] == b'0' {
            index += 1;
        } else {
            break;
        }
    }
    let from_index = if is_negative {
        dest[index - 1] = b'-';
        index - 1
    } else {
        index
    };
    let length = dest.len() - from_index;
    index = 0;
    while index < length {
        dest[index] = dest[from_index + index];
        index += 1;
    }
    &dest[..length]
}

impl I32 {
    const SIGN_FLAG: u8 = 0x20;

    pub fn from(inner: i32) -> Self {
        I32 {
            inner,
            first_byte: 0,
        }
    }

    pub fn unsafe_from(value: usize) -> Self {
        I32 {
            inner: value as i32,
            first_byte: 0,
        }
    }

    #[inline]
    pub fn get_length(&self) -> usize {
        decode_length(self.first_byte)
    }

    #[inline]
    pub fn is_fixed_size(&self) -> bool {
        is_fixed_size(self.first_byte)
    }

    pub fn to_str<'a>(&self, output: &'a mut [u8]) -> Option<&'a [u8]> {
        reset(output);
        if output.is_empty() {
            return None;
        }
        if self.inner == 0 {
            output[0] = b'0';
            return Some(&output[0..1]);
        }

        let num_length = if self.inner < 0 {
            output.len() - 1
        } else {
            output.len()
        };
        let mut raw_number = self.inner;
        let mut length = 0;
        while raw_number != 0 {
            if length >= num_length {
                return None;
            }
            let index = output.len() - length - 1;
            let number = raw_number % 10;
            output[index] = b'0'
                + if number < 0 {
                    (-number) as u8
                } else {
                    number as u8
                };
            raw_number /= 10;
            length += 1;
        }
        Some(trim(output, self.inner < 0))
    }

    fn decode_fixed_size<W: Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
        length: usize,
        from_index: usize,
    ) -> usize {
        if from_index == 0 {
            let is_positive = self.first_byte & I32::SIGN_FLAG == 0;
            if is_positive {
                self.inner = (((self.first_byte as u32) & MASK_MODE) << ((length - 1) * 8)) as i32;
            } else {
                self.inner =
                    (((self.first_byte as u32) | MASK_MODE_NEG) << ((length - 1) * 8)) as i32;
            }
            self.decode_i32(buffer, length, 1)
        } else {
            self.decode_i32(buffer, length, from_index)
        }
    }

    fn decode_i32<W: Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
        length: usize,
        from_index: usize,
    ) -> usize {
        let mut index = from_index;
        while !buffer.is_empty() && index < length {
            let byte = buffer.consume_byte().unwrap() as u32;
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
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<W: Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if buffer.is_empty() {
            return Ok(DecodeStage { ..*stage });
        }
        if stage.index == 0 {
            self.first_byte = buffer.consume_byte().unwrap();
        }
        let length = self.get_length();
        if length > 5 {
            return Err(DecodeError::InvalidSize);
        }

        let new_index = if self.is_fixed_size() {
            self.decode_fixed_size(buffer, length, stage.index as usize)
        } else {
            let from_index = if stage.index == 0 {
                stage.index + 1
            } else {
                stage.index
            };
            self.decode_i32(buffer, length, from_index as usize)
        };
        if new_index == length {
            Ok(DecodeStage::COMPLETE)
        } else {
            Ok(DecodeStage {
                step: stage.step,
                index: new_index as u16,
            })
        }
    }
}

#[cfg(test)]
pub mod tests {
    extern crate alloc;
    extern crate std;

    use crate::TempData;

    use super::*;
    use core::str::from_utf8;
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

        let mut temp_data = TempData::new();
        for item in arrays {
            let bytes = hex_to_bytes(item.0).unwrap();

            {
                let mut decoder = new_decoder::<I32>();
                let mut buffer = Buffer::new(&bytes, &mut temp_data);
                let result = decoder.decode(&mut buffer).unwrap();
                assert_eq!(result, Some(&I32::from(item.1)));
                assert!(decoder.stage.is_complete())
            }

            let mut length: usize = 0;
            let mut decoder = new_decoder::<I32>();

            while length < bytes.len() {
                let remain = bytes.len() - length;
                let size = random_usize(0, remain);
                let mut buffer = Buffer::new(&bytes[length..(length + size)], &mut temp_data);
                length += size;

                let result = decoder.decode(&mut buffer).unwrap();
                if length == bytes.len() {
                    assert_eq!(result, Some(&I32::from(item.1)));
                    assert!(decoder.stage.is_complete())
                } else {
                    assert_eq!(result, None);
                    assert_eq!(decoder.stage.index as usize, length);
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
