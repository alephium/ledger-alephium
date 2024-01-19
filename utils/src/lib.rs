#![no_std]

use core::char;
use core::num::Wrapping;

#[inline]
pub fn to_hex<const N: usize>(m: &[u8]) -> Result<[u8; N], ()> {
    if 2 * m.len() > N {
        return Err(());
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
    Ok(hex)
}

pub fn to_hex_fixed<const N: usize, const M: usize>(m: &[u8; N]) -> Result<[u8; M], ()> {
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
    Ok(hex)
}

pub fn djb_hash(data: &[u8]) -> i32 {
    let mut hash = Wrapping(5381 as i32);
    data.into_iter().for_each(|&byte| {
        hash = ((hash << 5) + hash) + Wrapping(byte as i32);
    });
    return hash.0;
}

pub fn xor_bytes(data: i32) -> u8 {
    let bytes = data.to_be_bytes();
    return bytes[0] ^ bytes[1] ^ bytes[2] ^ bytes[3];
}

pub fn deserialize_path(data: &[u8], path: &mut [u32; 5]) -> bool {
    // The path has to be 5 nodes
    if data.len() != 4 * 5 {
        return false;
    }

    for i in 0..5 {
        let offset = 4 * i;
        path[i] = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap());
    }

    return true;
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
}
