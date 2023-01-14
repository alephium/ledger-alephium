#![no_std]

use core::char;

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
        assert_eq!(input, x);
    }
}
