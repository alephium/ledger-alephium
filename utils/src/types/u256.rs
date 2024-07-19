use crate::buffer::{Buffer, Writable};
use crate::decode::*;
use crate::types::compact_integer::*;

use super::{reset, BigInt};

#[cfg_attr(test, derive(Debug))]
#[derive(Default, PartialEq, Clone)]
pub struct U256(pub BigInt);

impl Reset for U256 {
    fn reset(&mut self) {
        self.0.reset();
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
    const _1000_NANO_ALPH: u64 = 10_u64.pow((Self::ALPH_DECIMALS - Self::DECIMAL_PLACES) as u32);

    pub fn from_encoded_bytes(bytes: &[u8]) -> Self {
        let mut bs = [0u8; 33];
        bs[..bytes.len()].copy_from_slice(bytes);
        Self(BigInt { bytes: bs })
    }

    pub fn is_zero(&self) -> bool {
        self.0.get_length() == 1 && self.0.bytes.iter().all(|v| *v == 0)
    }

    pub fn to_u128(&self) -> Option<u128> {
        let length = self.0.get_length();
        if self.0.is_fixed_size() {
            Some(Self::decode_fixed_size(&self.0.bytes[..length]) as u128)
        } else if length <= 16 {
            let mut bytes = [0u8; 16];
            let tail = &self.0.bytes[1..length];
            bytes[(16 - tail.len())..].copy_from_slice(tail);
            Some(u128::from_be_bytes(bytes))
        } else {
            None
        }
    }

    pub fn multiply(&self, num: u32) -> Option<U256> {
        self.to_u128()
            .map(|value| U256::encode_u128(value * num as u128))
    }

    fn encode_fixed_bytes(n: u32) -> U256 {
        if n < 0x40 {
            U256::from_encoded_bytes(&[n as u8])
        } else if n < (0x40 << 8) {
            U256::from_encoded_bytes(&[((n >> 8) + 0x40) as u8, n as u8])
        } else if n < (0x40 << 24) {
            U256::from_encoded_bytes(&[
                ((n >> 24) + 0x40) as u8,
                (n >> 16) as u8,
                (n >> 8) as u8,
                n as u8,
            ])
        } else {
            panic!()
        }
    }

    fn encode_u128(value: u128) -> U256 {
        if value < (0x40 << 24) {
            U256::encode_fixed_bytes(value as u32)
        } else {
            let bytes = value.to_be_bytes();
            let mut index: usize = 0;
            for (i, &byte) in bytes.iter().enumerate() {
                if byte != 0 {
                    index = i;
                    break;
                }
            }
            let length = bytes.len() - index;
            let header: u8 = ((length - 4) as u8) | 0xc0;
            let mut bs = [0u8; 33];
            bs[0] = header;
            bs[1..(length + 1)].copy_from_slice(&bytes[index..]);
            Self(BigInt { bytes: bs })
        }
    }

    fn decode_fixed_size(bytes: &[u8]) -> u32 {
        assert!(bytes.len() <= 4);
        let mut result: u32 = ((bytes[0] as u32) & MASK_MODE) << ((bytes.len() - 1) * 8);
        let mut index = 1;
        while index < bytes.len() {
            let byte = bytes[index];
            result |= (byte as u32) << ((bytes.len() - index - 1) * 8);
            index += 1;
        }
        result
    }

    pub fn to_str<'a>(&self, output: &'a mut [u8]) -> Option<&'a [u8]> {
        if output.is_empty() {
            return None;
        }
        if self.is_zero() {
            output[0] = b'0';
            return Some(&output[..1]);
        }

        let length = self.0.get_length();
        let mut bytes = [0u8; 32];
        if self.0.is_fixed_size() {
            let value = Self::decode_fixed_size(&self.0.bytes[..length]);
            bytes[28..].copy_from_slice(&value.to_be_bytes());
        } else {
            bytes[(33 - length)..].copy_from_slice(&self.0.bytes[1..length])
        }
        let mut index = output.len();
        while !bytes.into_iter().all(|v| v == 0) {
            if index == 0 {
                return None;
            }
            index -= 1;
            let mut carry = 0u16;
            for element in &mut bytes {
                let v = (carry << 8) | (*element as u16);
                let rem = v % 10;
                *element = (v / 10) as u8;
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
        for (i, element) in output.iter_mut().enumerate().take(2 + pad_size) {
            if i == 1 {
                *element = b'.';
            } else {
                *element = b'0';
            }
        }
        return Some(trim(&output[..(2 + decimal_places)]));
    }

    fn is_less_than_1000_nano(&self) -> bool {
        if self.0.is_fixed_size() {
            return true;
        }
        let length = self.0.get_length();
        if length > 8 {
            return false;
        }
        let mut value: u64 = 0;
        let mut index = 1;
        while index < length {
            let byte = self.0.bytes[index];
            value = (value << 8) | (byte as u64);
            if value >= Self::_1000_NANO_ALPH {
                return false;
            }
            index += 1
        }
        true
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

        if self.is_less_than_1000_nano() {
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
        Some(&output[..total_size])
    }
}

impl RawDecoder for U256 {
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<W: Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        self.0.decode(buffer, stage)
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

    pub struct TestCase<'a>(pub Vec<u8>, pub &'a str);
    pub fn get_test_vector<'a>() -> [TestCase<'a>; 31] {
        [
            TestCase(hex_to_bytes("00").unwrap(), "0"),
            TestCase(hex_to_bytes("01").unwrap(), "1"),
            TestCase(hex_to_bytes("02").unwrap(), "2"),
            TestCase(hex_to_bytes("3e").unwrap(), "62"),
            TestCase(hex_to_bytes("3f").unwrap(), "63"),
            TestCase(hex_to_bytes("4040").unwrap(), "64"),
            TestCase(hex_to_bytes("4041").unwrap(), "65"),
            TestCase(hex_to_bytes("4042").unwrap(), "66"),
            TestCase(hex_to_bytes("7ffe").unwrap(), "16382"),
            TestCase(hex_to_bytes("7fff").unwrap(), "16383"),
            TestCase(hex_to_bytes("80004000").unwrap(), "16384"),
            TestCase(hex_to_bytes("80004001").unwrap(), "16385"),
            TestCase(hex_to_bytes("80004002").unwrap(), "16386"),
            TestCase(hex_to_bytes("bffffffe").unwrap(), "1073741822"),
            TestCase(hex_to_bytes("bfffffff").unwrap(), "1073741823"),
            TestCase(hex_to_bytes("c040000000").unwrap(), "1073741824"),
            TestCase(hex_to_bytes("c040000001").unwrap(), "1073741825"),
            TestCase(hex_to_bytes("c040000002").unwrap(), "1073741826"),
            TestCase(
                hex_to_bytes("c5010000000000000000").unwrap(),
                "18446744073709551616",
            ),
            TestCase(
                hex_to_bytes("c5010000000000000001").unwrap(),
                "18446744073709551617",
            ),
            TestCase(
                hex_to_bytes("c4ffffffffffffffff").unwrap(),
                "18446744073709551615",
            ),
            TestCase(
                hex_to_bytes("cd00000000000000ff00000000000000ff00").unwrap(),
                "1204203453131759529557760",
            ),
            TestCase(
                hex_to_bytes("cd0100000000000000000000000000000000").unwrap(),
                "340282366920938463463374607431768211456",
            ),
            TestCase(
                hex_to_bytes("cd0100000000000000000000000000000001").unwrap(),
                "340282366920938463463374607431768211457",
            ),
            TestCase(
                hex_to_bytes("ccffffffffffffffffffffffffffffffff").unwrap(),
                "340282366920938463463374607431768211455",
            ),
            TestCase(
                hex_to_bytes("d501000000000000000000000000000000000000000000000000").unwrap(),
                "6277101735386680763835789423207666416102355444464034512896",
            ),
            TestCase(
                hex_to_bytes("d501000000000000000000000000000000000000000000000001").unwrap(),
                "6277101735386680763835789423207666416102355444464034512897",
            ),
            TestCase(
                hex_to_bytes("d4ffffffffffffffffffffffffffffffffffffffffffffffff").unwrap(),
                "6277101735386680763835789423207666416102355444464034512895",
            ),
            TestCase(
                hex_to_bytes("dcffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
                    .unwrap(),
                "115792089237316195423570985008687907853269984665640564039457584007913129639935",
            ),
            TestCase(
                hex_to_bytes("dcfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe")
                    .unwrap(),
                "115792089237316195423570985008687907853269984665640564039457584007913129639934",
            ),
            TestCase(
                hex_to_bytes("dcfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd")
                    .unwrap(),
                "115792089237316195423570985008687907853269984665640564039457584007913129639933",
            ),
        ]
    }

    #[test]
    fn test_decode_u256() {
        let arrays = get_test_vector();
        let mut temp_data = TempData::new();
        for item in arrays {
            let bytes = item.0.as_slice();

            {
                let mut decoder = new_decoder::<U256>();
                let mut buffer = Buffer::new(bytes, &mut temp_data);
                let result = decoder.decode(&mut buffer).unwrap();
                assert!(result.is_some());
                let result = result.unwrap();
                let length = result.0.get_length();
                assert_eq!(bytes, &result.0.bytes[..length]);
                assert!(decoder.stage.is_complete());
            }

            let mut length: usize = 0;
            let mut decoder = new_decoder::<U256>();

            while length < bytes.len() {
                let remain = bytes.len() - length;
                let size = random_usize(0, remain);
                let mut buffer = Buffer::new(&bytes[length..(length + size)], &mut temp_data);
                length += size;

                let result = decoder.decode(&mut buffer).unwrap();
                if length == bytes.len() {
                    assert!(result.is_some());
                    let result = result.unwrap();
                    let length = result.0.get_length();
                    assert_eq!(bytes, &result.0.bytes[..length]);
                    assert!(decoder.stage.is_complete());
                } else {
                    assert_eq!(result, None);
                    assert_eq!(decoder.stage.index as usize, length);
                }
            }
        }
    }

    #[test]
    fn test_is_less_than_1000_nano_alph() {
        let u2560 = U256::encode_u128((U256::_1000_NANO_ALPH - 1) as u128);
        let u2561 = U256::encode_u128((U256::_1000_NANO_ALPH) as u128);
        let u2562 = U256::encode_u128((U256::_1000_NANO_ALPH + 1) as u128);

        assert!(u2560.is_less_than_1000_nano());
        assert!(!u2561.is_less_than_1000_nano());
        assert!(!u2562.is_less_than_1000_nano());
        assert!(!U256::encode_u128(u128::MAX).is_less_than_1000_nano())
    }

    #[test]
    fn test_multiply() {
        let min_gas_price = u128::pow(10, 11);
        let gas_amount = random_usize(1, 5000000) as u32;
        let fee = min_gas_price * (gas_amount as u128);
        let u256 = U256::encode_u128(min_gas_price)
            .multiply(gas_amount)
            .unwrap();
        assert!(u256.to_u128().unwrap() == fee);
        assert!(U256::encode_u128(u128::MAX).multiply(2).is_none());
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
            (U256::_1000_NANO_ALPH as u128, "0.000001"),
            ((10 as u128).pow(12), "0.000001"),
            ((U256::_1000_NANO_ALPH as u128) - 1, "<0.000001"),
            ((10 as u128).pow(13), "0.00001"),
            ((10 as u128).pow(14), "0.0001"),
            ((10 as u128).pow(17), "0.1"),
            ((10 as u128).pow(18), "1"),
            (alph("0.11111111111"), "0.111111"),
            (alph("111111.11111111"), "111111.111111"),
            (alph("1.010101"), "1.010101"),
            (alph("1.101010"), "1.10101"),
            (alph("1.9999999"), "1.999999"),
        ];
        for (number, str) in cases {
            let u256 = U256::encode_u128(number);
            let mut output = [0u8; 33];
            let result = u256.to_alph(&mut output);
            assert!(result.is_some());
            let expected = from_utf8(result.unwrap()).unwrap();
            let amount_str = String::from(str) + " ALPH";
            assert_eq!(amount_str, String::from(expected));
        }

        let test_vector = get_test_vector();
        let u256 = U256::from_encoded_bytes(&test_vector[test_vector.len() - 1].0);
        let mut output = [0u8; 33];
        assert!(u256.to_alph(&mut output).is_none());
    }

    #[test]
    fn test_to_str() {
        let test_vector = get_test_vector();

        for case in test_vector.iter() {
            let u256 = U256::from_encoded_bytes(&case.0);
            let mut output = [0u8; 78];
            let result = u256.to_str(&mut output).unwrap();
            let expected = from_utf8(&result).unwrap();
            assert_eq!(expected, case.1);
        }

        let case = &test_vector[test_vector.len() - 1];
        let u256 = U256::from_encoded_bytes(&case.0);
        let mut output = [0u8; 19];
        let result = u256.to_str(&mut output);
        assert!(result.is_none());
    }
}
