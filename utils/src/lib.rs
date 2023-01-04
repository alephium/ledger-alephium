#![no_std]

use core::char;

/// Convert to hex. Returns a static buffer of 64 bytes
#[inline]
pub fn to_hex(m: &[u8]) -> Result<[u8; 64], ()> {
    if 2 * m.len() > 64 {
        return Err(());
    }
    let mut hex = [0u8; 64];
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

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;

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
}
