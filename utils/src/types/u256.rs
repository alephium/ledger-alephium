use crate::buffer::Buffer;
use crate::decode::*;
use crate::types::compact_integer::*;

use super::reset;

#[cfg_attr(test, derive(Debug))]
#[derive(Default)]
pub struct U256 {
    pub inner: [u64; 4],
    first_byte: u8,
}

impl Reset for U256 {
    fn reset(&mut self) {
        self.inner = [0; 4];
        self.first_byte = 0;
    }
}

impl PartialEq for U256 {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

fn trim(dest: &[u8]) -> &[u8] {
    let mut index = dest.len() - 1;
    while index != 0 {
        if dest[index] == b'0' {
            index -= 1;
        } else {
            break;
        }
    }
    if dest[index] == b'.' {
        &dest[..index]
    } else {
        &dest[..index + 1]
    }
}

impl U256 {
    const ALPH_DECIMALS: usize = 18;
    const DECIMAL_PLACES: usize = 6;
    const ALPH_MIN: u64 = (10 as u64).pow((Self::ALPH_DECIMALS - Self::DECIMAL_PLACES) as u32);

    fn less_than_alph_min(&self) -> bool {
        let array = &self.inner;
        array[0] == 0 && array[1] == 0 && array[2] == 0 && array[3] < Self::ALPH_MIN
    }

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

    #[inline]
    fn is_reviewable(&self) -> bool {
        self.inner[0] == 0 && self.inner[1] == 0
    }

    pub fn to_u128(&self) -> Option<u128> {
        if self.inner[0] == 0 && self.inner[1] == 0 {
            return Some(((self.inner[2] as u128) << 64) | (self.inner[3] as u128));
        }
        return None;
    }

    pub fn to_str<'a>(&self, output: &'a mut [u8]) -> Option<&'a [u8]> {
        if output.len() == 0 || !self.is_reviewable() {
            return None;
        }
        if self.is_zero() {
            output[0] = b'0';
            return Some(&output[..1]);
        }
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&self.inner[2].to_be_bytes());
        bytes[8..].copy_from_slice(&self.inner[3].to_be_bytes());
        let mut index = output.len();
        while !bytes.into_iter().all(|v| v == 0) {
            if index == 0 {
                return None;
            }
            index -= 1;
            let mut carry = 0u16;
            for i in 0..16 {
                let v = (carry << 8) | (bytes[i] as u16);
                let rem = v % 10;
                bytes[i] = (v / 10) as u8;
                carry = rem;
            }
            output[index] = b'0' + (carry as u8);
        }
        output.copy_within(index..output.len(), 0);
        Some(&output[..(output.len() - index)])
    }

    fn to_str_with_decimals<'a>(
        &self,
        output: &'a mut [u8],
        decimals: usize,
        decimal_places: usize,
    ) -> Option<&'a [u8]> {
        reset(output);
        let str = self.to_str(output)?;
        let str_length = str.len();
        if decimals == 0 {
            return Some(&output[..str_length]);
        }

        if str_length > decimals {
            let decimal_index = str_length - decimals;
            output.copy_within(decimal_index..str_length, decimal_index + 1);
            output[decimal_index] = b'.';
            return Some(trim(&output[..(decimal_index + decimal_places + 1)]));
        }

        let pad_size = decimals - str_length;
        output.copy_within(0..str_length, 2 + pad_size);
        for i in 0..(2 + pad_size) {
            if i == 1 {
                output[i] = b'.';
            } else {
                output[i] = b'0';
            }
        }
        return Some(trim(&output[..(2 + decimal_places)]));
    }

    pub fn to_alph<'a>(&self, output: &'a mut [u8]) -> Option<&'a [u8]> {
        reset(output);
        let postfix = b" ALPH";
        if self.is_zero() {
            output[0] = b'0';
            let total_size = 1 + postfix.len();
            output[1..total_size].copy_from_slice(postfix);
            return Some(&output[..total_size]);
        }

        if self.less_than_alph_min() {
            let str = b"<0.000001";
            let total_size = str.len() + postfix.len();
            if output.len() < total_size {
                return None;
            }
            output[..str.len()].copy_from_slice(str);
            output[str.len()..total_size].copy_from_slice(postfix);
            return Some(&output[..total_size]);
        }

        if output.len() < 28 + postfix.len() {
            // max ALPH amount
            return None;
        }

        let str = self.to_str_with_decimals(output, Self::ALPH_DECIMALS, Self::DECIMAL_PLACES)?;
        let str_length = str.len();
        let total_size = str_length + postfix.len();
        output[str_length..total_size].copy_from_slice(postfix);
        return Some(&output[..total_size]);
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
    use crate::types::u256::U256;
    use crate::{decode::*, TempData};
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
        let mut temp_data = TempData::new();
        for item in arrays {
            let bytes = hex_to_bytes(item.0).unwrap();

            {
                let mut decoder = new_decoder::<U256>();
                let mut buffer = Buffer::new(&bytes, &mut temp_data).unwrap();
                let result = decoder.decode(&mut buffer).unwrap();
                assert_eq!(result, Some(&item.1));
                assert!(decoder.stage.is_complete())
            }

            let mut length: usize = 0;
            let mut decoder = new_decoder::<U256>();

            while length < bytes.len() {
                let remain = bytes.len() - length;
                let size = random_usize(0, remain);
                let mut buffer =
                    Buffer::new(&bytes[length..(length + size)], &mut temp_data).unwrap();
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
            (U256::ALPH_MIN as u128, "0.000001"),
            ((10 as u128).pow(12), "0.000001"),
            ((U256::ALPH_MIN as u128) - 1, "<0.000001"),
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
            let mut output = [0u8; 33];
            let result = u256.to_alph(&mut output);
            assert!(result.is_some());
            let expected = from_utf8(result.unwrap()).unwrap();
            let amount_str = String::from(str) + " ALPH";
            assert_eq!(amount_str, String::from(expected));
        }

        let mut output = [0u8; 33];
        let value = U256::from([0, 1, 0, 0]);
        assert!(value.to_alph(&mut output).is_none());
    }

    #[test]
    fn test_to_str() {
        let cases = [
            (U256::from_u32(0), "0"),
            (U256::from_u32(1), "1"),
            (U256::from_u32(100), "100"),
            (U256::from_u32(12345), "12345"),
            (U256::from_u32(123456), "123456"),
            (U256::from_u32(u32::MAX - 1), "4294967294"),
            (U256::from_u32(u32::MAX), "4294967295"),
            (U256::from_u64(1234567890), "1234567890"),
            (U256::from_u64(u64::MAX - 1), "18446744073709551614"),
            (U256::from_u64(u64::MAX), "18446744073709551615"),
            (
                U256::from_u128(u128::MAX),
                "340282366920938463463374607431768211455",
            ),
        ];

        for (u256, str) in cases {
            let mut output = [0u8; 39];
            let result = u256.to_str(&mut output).unwrap();
            let expected = from_utf8(&result).unwrap();
            assert_eq!(expected, str);
        }

        let u256 = U256::from_u64(u64::MAX);
        let mut output = [0u8; 19];
        let result = u256.to_str(&mut output);
        assert!(result.is_none());
    }
}
