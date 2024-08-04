#![no_std]

pub mod base58;
pub mod buffer;
pub mod decode;
#[cfg(test)]
pub mod temp_data;
pub mod types;

use core::char;
use core::num::Wrapping;
#[cfg(test)]
pub use temp_data::TempData;

#[inline]
pub fn to_hex<const N: usize>(m: &[u8]) -> Option<[u8; N]> {
    if 2 * m.len() > N {
        return None;
    }
    let mut hex = [0u8; N];
    let mut i = 0;
    for c in m {
        let c0 = char::from_digit((c >> 4).into(), 16).unwrap();
        let c1 = char::from_digit((c & 0xf).into(), 16).unwrap();
        hex[i] = c0 as u8;
        hex[i + 1] = c1 as u8;
        i += 2;
    }
    Some(hex)
}

pub fn to_hex_fixed<const N: usize, const M: usize>(m: &[u8; N]) -> [u8; M] {
    assert!(M == 2 * N);
    let mut hex = [0u8; M];
    let mut i = 0;
    for c in m {
        let c0 = char::from_digit((c >> 4).into(), 16).unwrap();
        let c1 = char::from_digit((c & 0xf).into(), 16).unwrap();
        hex[i] = c0 as u8;
        hex[i + 1] = c1 as u8;
        i += 2;
    }
    hex
}

// This is a non-critical hash function and collision is totally fine
pub fn djb_hash(data: &[u8]) -> i32 {
    let mut hash = Wrapping(5381_i32);
    data.iter().for_each(|&byte| {
        hash = ((hash << 5) + hash) + Wrapping(byte as i32);
    });
    hash.0
}

pub fn xor_bytes(data: i32) -> u8 {
    let bytes = data.to_be_bytes();
    bytes[0] ^ bytes[1] ^ bytes[2] ^ bytes[3]
}

pub const PATH_LENGTH: usize = 5;

// Deserialize a path from a byte array
pub fn deserialize_path<T>(data: &[u8], path: &mut [u32; 5], t: T) -> Result<(), T> {
    // The path has to be 5 nodes
    if data.len() != 4 * PATH_LENGTH {
        return Err(t);
    }

    for i in 0..PATH_LENGTH {
        let offset = 4 * i;
        path[i] = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap());
    }

    Ok(())
}

// If the group number is 0, the target group must also be 0, meaning all groups are allowed
// If the group number is not 0, the target group must be less than the group number
pub fn check_group<T>(group_num: u8, target_group: u8, t: T) -> Result<(), T> {
    if group_num == 0 && target_group == 0 {
        return Ok(());
    }
    if target_group >= group_num {
        return Err(t);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use core::str::FromStr;
    use std::string::String;
    use std::vec::Vec;

    pub fn to_hex_string(m: &[u8]) -> String {
        use core::str::from_utf8;

        let hex = to_hex::<64>(m).unwrap();
        let result = from_utf8(&hex).unwrap();
        String::from_str(result).unwrap()
    }

    pub fn from_hex_string(s: &str) -> Vec<u8> {
        assert!(s.len() % 2 == 0);
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect::<Vec<u8>>()
    }

    #[test]
    fn test_to_hex() {
        let input: [u8; 32] = core::array::from_fn(|i| i as u8);
        let output: [u8; 64] = core::array::from_fn(|i| {
            let m = input[i / 2];
            let c = if i % 2 == 0 { m >> 4 } else { m & 0xf };
            char::from_digit(c.into(), 16).unwrap() as u8
        });
        assert_eq!(to_hex(&input).unwrap(), output);
    }

    #[test]
    fn test_hex_string() {
        let input = "0123456789abcdef";
        let x = to_hex_string(&from_hex_string(input));
        assert_eq!(input, &x.as_str()[0..16]);
    }

    #[test]
    fn test_djb_hash() {
        assert_eq!(djb_hash(&[]), 5381);
        assert_eq!(djb_hash(&[97]), 177670);
        assert_eq!(djb_hash(&[122]), 177695);
        assert_eq!(djb_hash(&[102, 111, 111]), 193491849);
        assert_eq!(djb_hash(&[98, 97, 114]), 193487034);
        assert_eq!(djb_hash(&[49, 50, 51, 52, 53, 54, 55, 56, 57]), 902675330);
    }

    #[test]
    fn test_xor_bytes() {
        assert_eq!(xor_bytes(-1), 0);
        assert_eq!(xor_bytes(-1909601881), 205);
        assert_eq!(xor_bytes(-2147483648), 128);
        assert_eq!(xor_bytes(-1071872007), 162);
        assert_eq!(xor_bytes(1), 1);
        assert_eq!(xor_bytes(-113353554), 53);
        assert_eq!(xor_bytes(2147483647), 128);
        assert_eq!(xor_bytes(2147483647), 128);
        assert_eq!(xor_bytes(-2146081904), 102);
        assert_eq!(xor_bytes(1226685873), 88);
    }

    #[test]
    fn test_deserialize_path() {
        assert_eq!(deserialize_path(&[], &mut [0; 5], ()), Err(()));
        assert_eq!(deserialize_path(&[0; 19], &mut [0; 5], ()), Err(()));
        assert_eq!(deserialize_path(&[0; 20], &mut [0; 5], ()), Ok(()));
        assert_eq!(deserialize_path(&[0; 21], &mut [0; 5], ()), Err(()));

        let mut path = [0; 5];
        let _ = deserialize_path(&[1; 20], &mut path, ());
        assert_eq!(&path, &[0x01010101; 5]);
    }

    #[test]
    fn test_check_group() {
        // When group_num is 0, target_group must be 0
        assert_eq!(check_group(0, 0, ()), Ok(()));
        assert_eq!(check_group(0, 1, ()), Err(()));

        // When group_num is not 0, target_group must be less than group_num
        assert_eq!(check_group(1, 0, ()), Ok(()));
        assert_eq!(check_group(1, 1, ()), Err(()));
        assert_eq!(check_group(1, 2, ()), Err(()));
        assert_eq!(check_group(2, 0, ()), Ok(()));
        assert_eq!(check_group(2, 1, ()), Ok(()));
        assert_eq!(check_group(2, 2, ()), Err(()));
        assert_eq!(check_group(2, 3, ()), Err(()));
    }
}
