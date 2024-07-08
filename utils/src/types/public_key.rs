use crate::buffer::Buffer;
use crate::decode::*;
use crate::fixed_size_bytes;

fixed_size_bytes!(PublicKey, 33);

#[cfg(test)]
mod tests {
    extern crate std;

    use super::PublicKey;
    use crate::buffer::Buffer;
    use crate::decode::{new_decoder, Decoder};
    use crate::types::byte32::tests::gen_bytes;
    use crate::TempData;
    use std::vec;

    #[test]
    fn test_decode_public_key() {
        let mut bytes = vec![0u8; 0];
        let mut decoder = new_decoder::<PublicKey>();
        let mut temp_data = TempData::new();

        while bytes.len() < PublicKey::ENCODED_LENGTH {
            let data = gen_bytes(0, PublicKey::ENCODED_LENGTH * 2);
            let mut buffer = Buffer::new(data.as_slice(), &mut temp_data).unwrap();
            bytes.extend(&data);

            let result = decoder.decode(&mut buffer);
            if bytes.len() < PublicKey::ENCODED_LENGTH {
                assert_eq!(result, Ok(None));
            } else {
                let array: [u8; 33] = bytes.as_slice()[0..PublicKey::ENCODED_LENGTH]
                    .try_into()
                    .unwrap();
                assert_eq!(result, Ok(Some(&PublicKey::from_bytes(array))));
            }
        }
    }
}
