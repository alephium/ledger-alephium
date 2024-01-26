use crate::buffer::Buffer;
use crate::decode::*;
use crate::types::compact_integer::*;

use super::{extend_slice, reset};

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

fn trim<'a, const NUM: usize>(dest: &'a mut [u8; NUM]) -> &'a [u8] {
    let mut index = dest.len() - 1;
    while index != 0 {
        if dest[index] == b'0' {
            index -= 1;
        } else {
            break;
        }
    }
    if dest[index] == b'.' {
        &dest[0..(index)]
    } else {
        &dest[0..(index + 1)]
    }
}

impl U256 {
    const ALPH_DECIMALS: usize = 18;
    const DECIMAL_PLACES: usize = 6;
    const ALPH_MIN: u128 = (10 as u128).pow((Self::ALPH_DECIMALS - Self::DECIMAL_PLACES) as u32);

    #[inline]
    pub fn get_length(&self) -> usize {
        decode_length(self.first_byte)
    }

    #[inline]
    pub fn is_fixed_size(&self) -> bool {
        is_fixed_size(self.first_byte)
    }

    pub fn is_zero(&self) -> bool {
        self.inner.iter().all(|x| *x == 0)
    }

    pub fn from(array: [u64; 4]) -> U256 {
        U256 {
            inner: array,
            first_byte: 0,
        }
    }

    pub fn from_u32(value: u32) -> U256 {
        Self::from_u64(value as u64)
    }

    pub fn from_u64(value: u64) -> U256 {
        U256 {
            inner: [0, 0, 0, value],
            first_byte: 0,
        }
    }

    #[cfg(test)]
    pub fn from_u128(value: u128) -> U256 {
        U256 {
            inner: [
                0,
                0,
                ((value >> 64) & (u64::MAX as u128)) as u64,
                (value & (u64::MAX as u128)) as u64,
            ],
            first_byte: 0,
        }
    }

    pub fn to_u128(&self) -> Option<u128> {
        if self.inner[0] != 0 || self.inner[1] != 0 {
            return None;
        }
        let result = ((self.inner[2] as u128) << 64) | (self.inner[3] as u128);
        Some(result)
    }

    pub fn to_alph<'a, const NUM: usize>(&self, output: &'a mut [u8; NUM]) -> Option<&'a [u8]> {
        reset(output);

        if self.is_zero() {
            output[0] = b'0';
            return Some(&output[..1]);
        }

        let mut raw_amount = self.to_u128()?;
        if raw_amount < Self::ALPH_MIN {
            extend_slice(output, 0, b"<0.000001");
            return Some(trim(output));
        }

        let mut bytes = [b'0'; 28];
        let mut length = 0;
        while raw_amount > 0 {
            if length >= bytes.len() {
                return None;
            }
            let index = bytes.len() - length - 1;
            bytes[index] = b'0' + ((raw_amount % 10) as u8);
            raw_amount = raw_amount / 10;
            length += 1;
        }

        let str_length = if length > Self::ALPH_DECIMALS {
            length - 18 + 1 + Self::DECIMAL_PLACES
        } else {
            2 + Self::DECIMAL_PLACES
        };
        if str_length > output.len() {
            return None;
        }

        let decimal_index = bytes.len() - Self::ALPH_DECIMALS;
        let from_index = if length > Self::ALPH_DECIMALS {
            let str = &bytes[(bytes.len() - length)..decimal_index];
            extend_slice(output, 0, str)
        } else {
            extend_slice(output, 0, b"0")
        };
        extend_slice(output, from_index, &[b'.']);
        extend_slice(
            output,
            from_index + 1,
            &bytes[decimal_index..(decimal_index + Self::DECIMAL_PLACES)],
        );
        Some(trim(output))
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

    fn decode_multi_bytes(
        &mut self,
        buffer: &mut Buffer,
        length: usize,
        from_index: usize,
    ) -> usize {
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
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if buffer.is_empty() {
            return Ok(DecodeStage { ..*stage });
        }
        if stage.index == 0 {
            self.first_byte = buffer.next_byte().unwrap();
        }
        let length = self.get_length();
        let new_index = if self.is_fixed_size() {
            if stage.index == 0 {
                self.inner[3] =
                    (((self.first_byte as u32) & MASK_MODE) << ((length - 1) * 8)) as u64;
                self.decode_u32(buffer, length, (stage.index as usize) + 1)
            } else {
                self.decode_u32(buffer, length, stage.index as usize)
            }
        } else {
            let from_index = if stage.index == 0 {
                stage.index + 1
            } else {
                stage.index
            };
            if length == 5 {
                self.decode_u32(buffer, length, from_index as usize)
            } else {
                self.decode_multi_bytes(buffer, length, from_index as usize)
            }
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
    extern crate std;

    use crate::buffer::Buffer;
    use crate::decode::*;
    use crate::types::u256::U256;
    use core::str::from_utf8;
    use rand::Rng;
    use std::string::String;
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
            TestCase(
                "cd00000000000000ff00000000000000ff00",
                U256::from([0, 0, 0xff00, 0xff00]),
            ),
            TestCase(
                "cd0100000000000000000000000000000001",
                U256::from([0, 1, 0, 1]),
            ),
            TestCase(
                "cd0100000000000000000000000000000000",
                U256::from([0, 1, 0, 0]),
            ),
            TestCase(
                "cd0100000000000000000000000000000001",
                U256::from([0, 1, 0, 1]),
            ),
            TestCase(
                "ccffffffffffffffffffffffffffffffff",
                U256::from([0, 0, u64_max, u64_max]),
            ),
            TestCase(
                "d501000000000000000000000000000000000000000000000000",
                U256::from([1, 0, 0, 0]),
            ),
            TestCase(
                "d501000000000000000000000000000000000000000000000001",
                U256::from([1, 0, 0, 1]),
            ),
            TestCase(
                "d4ffffffffffffffffffffffffffffffffffffffffffffffff",
                U256::from([0, u64_max, u64_max, u64_max]),
            ),
            TestCase(
                "dcffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                U256::from([u64_max; 4]),
            ),
            TestCase(
                "dcfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe",
                U256::from([u64_max, u64_max, u64_max, u64_max - 1]),
            ),
            TestCase(
                "dcfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd",
                U256::from([u64_max, u64_max, u64_max, u64_max - 2]),
            ),
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
                let mut buffer = Buffer::new(&bytes[length..(length + size)]).unwrap();
                length += size;

                let result = decoder.decode(&mut buffer).unwrap();
                if length == bytes.len() {
                    assert_eq!(result, Some(&item.1));
                    assert!(decoder.stage.is_complete())
                } else {
                    assert_eq!(result, None);
                    assert_eq!(decoder.stage.index as usize, length);
                }
            }
        }
    }

    #[test]
    fn test_to_alph() {
        let alph = |str: &str| {
            let index_opt = str.find('.');
            let mut result_str = String::new();
            if index_opt.is_none() {
                result_str.extend(str.chars());
                let pad: String = std::iter::repeat('0').take(U256::ALPH_DECIMALS).collect();
                result_str.extend(pad.chars());
            } else {
                let index = index_opt.unwrap();
                let pad_size = U256::ALPH_DECIMALS - (str.len() - index_opt.unwrap()) + 1;
                result_str.extend(str[0..index].chars());
                result_str.extend(str[(index + 1)..].chars());
                let pad: String = std::iter::repeat('0').take(pad_size).collect();
                result_str.extend(pad.chars());
            }
            result_str.parse::<u128>().unwrap()
        };

        let cases = [
            (0, "0"),
            (U256::ALPH_MIN, "0.000001"),
            ((10 as u128).pow(12), "0.000001"),
            (U256::ALPH_MIN - 1, "<0.000001"),
            ((10 as u128).pow(13), "0.00001"),
            ((10 as u128).pow(14), "0.0001"),
            ((10 as u128).pow(17), "0.1"),
            ((10 as u128).pow(17), "0.1"),
            ((10 as u128).pow(18), "1"),
            (alph("0.11111111111"), "0.111111"),
            (alph("111111.11111111"), "111111.111111"),
            (alph("1.010101"), "1.010101"),
            (alph("1.101010"), "1.10101"),
            (alph("1.9999999"), "1.999999"),
        ];
        for (number, str) in cases {
            let u256 = U256::from_u128(number);
            let mut output = [0u8; 17];
            let result = u256.to_alph(&mut output);
            assert!(result.is_some());
            let expected = from_utf8(result.unwrap()).unwrap();
            assert_eq!(*str, *expected);
        }

        let mut output = [0u8; 17];
        let value = U256::from([0, 1, 0, 0]);
        assert!(value.to_alph(&mut output).is_none());
    }
}
