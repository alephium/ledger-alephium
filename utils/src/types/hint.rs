use crate::buffer::Buffer;
use crate::decode::*;
use crate::fixed_bytes;

fixed_bytes!(Hint, 4);

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{new_decoder, Hint};
    use crate::buffer::Buffer;
    use crate::decode::Decoder;
    use crate::TempData;
    use rand::Rng;
    use std::vec;
    use std::vec::Vec;

    fn gen_bytes(min_length: usize, max_length: usize) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let length = rng.gen_range(min_length..=max_length);
        let mut random_bytes = vec![0u8; length];
        rng.fill(&mut random_bytes[..]);
        random_bytes
    }

    #[test]
    fn test_decode_hint() {
        let mut temp_data = TempData::new();
        let mut bytes = vec![0u8; 0];
        let mut decoder = new_decoder::<Hint>();

        while bytes.len() < Hint::ENCODED_LENGTH {
            let data = gen_bytes(0, Hint::ENCODED_LENGTH * 2);
            let mut buffer = Buffer::new(data.as_slice(), &mut temp_data).unwrap();
            bytes.extend(&data);

            let result = decoder.decode(&mut buffer);
            if bytes.len() < Hint::ENCODED_LENGTH {
                assert_eq!(result, Ok(None));
            } else {
                let array: [u8; 4] = bytes.as_slice()[0..Hint::ENCODED_LENGTH]
                    .try_into()
                    .unwrap();
                assert_eq!(result, Ok(Some(&Hint::from_bytes(array))));
            }
        }
    }
}
