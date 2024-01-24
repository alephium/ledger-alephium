// from: https://github.com/Nullus157/bs58-rs/blob/main/src/encode.rs
const ALPHABET: &'static [u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

pub fn base58_encode<'a>(input: &[u8], output: &'a mut [u8]) -> Option<&'a [u8]> {
    let mut index = 0;
    for &val in input {
        let mut carry = val as usize;
        for byte in &mut output[..index] {
            carry += (*byte as usize) << 8;
            *byte = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            if index == output.len() {
                return None;
            }
            output[index] = (carry % 58) as u8;
            index += 1;
            carry /= 58;
        }
    }

    for _ in input.into_iter().take_while(|&&v| v == 0) {
        if index == output.len() {
            return None;
        }
        output[index] = 0;
        index += 1;
    }

    for val in &mut output[..index] {
        *val = ALPHABET[*val as usize];
    }

    output[..index].reverse();
    Some(&output[..index])
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::base58_encode;
    use crate::types::u256::tests::hex_to_bytes;
    use alloc::str::from_utf8;

    #[test]
    fn test_base58_encode() {
        let cases: [(&[u8], &str); 8] = [
            (b"", ""),
            (b"abc", "ZiCa"),
            (b"\0abc", "1ZiCa"),
            (b"\0\0abc", "11ZiCa"),
            (
                &hex_to_bytes("00bd8813e79baa5fa1874ca8b70877d1b044e220ecd34a60eca3ba15fc36b378e7")
                    .unwrap(),
                "1DkrQMni2h8KYpvY8t7dECshL66gwnxiR5uD2Udxps6og",
            ),
            (
                &hex_to_bytes("001dd2aa371711d1faea1c96d395f08eb94de1f388993e8be3f4609dc327ab513a")
                    .unwrap(),
                "131R8ufDhcsu6SRztR9D3m8GUzkWFUPfT78aQ6jgtgzob",
            ),
            (
                &hex_to_bytes("02798e9e137aec7c2d59d9655b4ffa640f301f628bf7c365083bb255f6aa5f89ef")
                    .unwrap(),
                "je9CrJD444xMSGDA2yr1XMvugoHuTc6pfYEaPYrKLuYa",
            ),
            (
                &hex_to_bytes("02e5d64f886664c58378d41fe3b8c29dd7975da59245a4a6bf92c3a47339a9a0a9")
                    .unwrap(),
                "rvpeCy7GhsGHq8n6TnB1LjQh4xn1FMHJVXnsdZAniKZA",
            ),
        ];
        for (bytes, str) in cases {
            let mut result = [0; 50];
            let result = base58_encode(bytes, &mut result);
            assert!(result.is_some());
            let expected = from_utf8(result.unwrap()).unwrap();
            assert_eq!(*expected, *str);
        }
    }
}
