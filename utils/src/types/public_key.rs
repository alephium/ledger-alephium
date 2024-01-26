use crate::buffer::Buffer;
use crate::decode::*;

#[cfg_attr(test, derive(Debug))]
#[derive(PartialEq)]
pub struct PublicKey(pub [u8; 33]);

impl PublicKey {
    const ENCODED_LENGTH: usize = 33;

    pub fn from_bytes(bytes: [u8; 33]) -> Self {
        PublicKey(bytes)
    }
}

impl Default for PublicKey {
    fn default() -> Self {
        PublicKey([0; 33])
    }
}

impl RawDecoder for PublicKey {
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        let remain = PublicKey::ENCODED_LENGTH - (stage.index as usize);
        let mut idx: usize = 0;
        while !buffer.is_empty() && idx < remain {
            self.0[(stage.index as usize) + idx] = buffer.next_byte().unwrap();
            idx += 1;
        }
        let new_index = (stage.index as usize) + idx;
        if new_index == PublicKey::ENCODED_LENGTH {
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
mod tests {
    extern crate std;

    use super::PublicKey;
    use crate::buffer::Buffer;
    use crate::decode::{new_decoder, Decoder};
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
    fn test_decode_public_key() {
        let mut bytes = vec![0u8; 0];
        let mut decoder = new_decoder::<PublicKey>();

        while bytes.len() < PublicKey::ENCODED_LENGTH {
            let data = gen_bytes(0, PublicKey::ENCODED_LENGTH * 2);
            let mut buffer = Buffer::new(data.as_slice()).unwrap();
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
