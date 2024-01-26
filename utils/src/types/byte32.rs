use crate::buffer::Buffer;
use crate::decode::*;

#[cfg_attr(test, derive(Debug))]
#[derive(Default, PartialEq)]
pub struct Byte32(pub [u8; 32]);

impl Byte32 {
    const ENCODED_LENGTH: usize = 32;

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Byte32(bytes)
    }
}

impl RawDecoder for Byte32 {
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        let remain = Byte32::ENCODED_LENGTH - (stage.index as usize);
        let mut idx: usize = 0;
        while !buffer.is_empty() && idx < remain {
            self.0[(stage.index as usize) + idx] = buffer.next_byte().unwrap();
            idx += 1;
        }
        let new_index = (stage.index as usize) + idx;
        if new_index == Byte32::ENCODED_LENGTH {
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
pub mod tests {
    extern crate std;

    use super::Byte32;
    use crate::buffer::Buffer;
    use crate::decode::{new_decoder, Decoder};
    use rand::Rng;
    use std::vec;
    use std::vec::Vec;

    pub fn gen_bytes(min_length: usize, max_length: usize) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let length = rng.gen_range(min_length..=max_length);
        let mut random_bytes = vec![0u8; length];
        rng.fill(&mut random_bytes[..]);
        random_bytes
    }

    #[test]
    fn test_decode_byte32() {
        let mut bytes = vec![0u8; 0];
        let mut decoder = new_decoder::<Byte32>();

        while bytes.len() < Byte32::ENCODED_LENGTH {
            let data = gen_bytes(0, Byte32::ENCODED_LENGTH * 2);
            let mut buffer = Buffer::new(data.as_slice()).unwrap();
            bytes.extend(&data);

            let result = decoder.decode(&mut buffer);
            if bytes.len() < Byte32::ENCODED_LENGTH {
                assert_eq!(result, Ok(None));
            } else {
                let array: [u8; 32] = bytes.as_slice()[0..Byte32::ENCODED_LENGTH]
                    .try_into()
                    .unwrap();
                assert_eq!(result, Ok(Some(&Byte32::from_bytes(array))));
            }
        }
    }
}
