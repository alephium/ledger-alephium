// from: https://github.com/Nullus157/bs58-rs/blob/main/src/encode.rs
const ALPHABET: &'static [u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

pub fn base58_encode_inputs<'a>(inputs: &[&[u8]], output: &'a mut [u8]) -> Option<&'a [u8]> {
    let mut index = 0;
    for &input in inputs {
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
    }

    'outer: for &input in inputs {
        for &val in input {
            if val != 0 {
                break 'outer;
            }
            if index == output.len() {
                return None;
            }
            output[index] = 0;
            index += 1;
        }
    }

    for val in &mut output[..index] {
        *val = ALPHABET[*val as usize];
    }

    output[..index].reverse();
    Some(&output[..index])
}

pub fn base58_encode<'a>(input: &[u8], output: &'a mut [u8]) -> Option<&'a [u8]> {
    base58_encode_inputs(&[input], output)
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    extern crate std;
    use super::base58_encode;
    use crate::{base58::base58_encode_inputs, types::u256::tests::hex_to_bytes};
    use alloc::str::from_utf8;
    use std::vec;
    use std::vec::Vec;

    #[test]
    fn test_base58_encode() {
        let cases: [(&[u8], &str); 11] = [
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
            (
                &hex_to_bytes("0102a3cd757be03c7dac8d48bf79e2a7d6e735e018a9c054b99138c7b29738c437ecef51c98556924afa1cd1a8026c3d2d33ee1d491e1fe77c73a75a2d0129f0619501")
                    .unwrap(),
                "2jjvDdgGjC6X9HHMCMHohVfvp1uf3LHQrAGWaufR17P7AFwtxodTxSktqKc2urNEtaoUCy5xXpBUwpZ8QM8Q3e5BYCx",
            ),
            (
                &hex_to_bytes("0102a3cd757be03c7dac8d48bf79e2a7d6e735e018a9c054b99138c7b29738c437ecef51c98556924afa1cd1a8026c3d2d33ee1d491e1fe77c73a75a2d0129f0619502")
                    .unwrap(),
                "2jjvDdgGjC6X9HHMCMHohVfvp1uf3LHQrAGWaufR17P7AFwtxodTxSktqKc2urNEtaoUCy5xXpBUwpZ8QM8Q3e5BYCy",
            ),
            (
                &hex_to_bytes("0103a3cd757be03c7dac8d48bf79e2a7d6e735e018a9c054b99138c7b29738c437ecef51c98556924afa1cd1a8026c3d2d33ee1d491e1fe77c73a75a2d0129f061951dd2aa371711d1faea1c96d395f08eb94de1f388993e8be3f4609dc327ab513a02")
                    .unwrap(),
                "X3RMnvb8h3RFrrbBraEouAWU9Ufu4s2WTXUQfLCvDtcmqCWRwkVLc69q2NnwYW2EMwg4QBN2UopkEmYLLLgHP9TQ38FK15RnhhEwguRyY6qCuAoRfyjHRnqYnTvfypPgD7w1ku",
            ),
        ];
        for (bytes, str) in cases {
            let mut output = [0; 150];
            let result = base58_encode(bytes, &mut output);
            assert!(result.is_some());
            let expected = from_utf8(result.unwrap()).unwrap();
            assert_eq!(*expected, *str);

            let input_slices: Vec<Vec<u8>> = bytes.iter().map(|&byte| vec![byte]).collect();
            let inputs: Vec<&[u8]> = input_slices.iter().map(|v| v.as_slice()).collect();
            let result = base58_encode_inputs(&inputs, &mut output);
            assert!(result.is_some());
            let expected = from_utf8(result.unwrap()).unwrap();
            assert_eq!(*expected, *str);
        }
    }
}
