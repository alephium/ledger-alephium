use super::*;
use crate::buffer::{Buffer, Writable};
use crate::decode::*;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct Token {
    pub id: Hash,
    pub amount: U256,
}

impl Reset for Token {
    fn reset(&mut self) {
        self.id.reset();
        self.amount.reset();
    }
}

impl Token {
    pub fn from(id: Hash, amount: U256) -> Self {
        Token { id, amount }
    }
}

impl RawDecoder for Token {
    fn step_size(&self) -> u16 {
        2
    }

    fn decode<'a, W: Writable>(
        &mut self,
        buffer: &mut Buffer<'a, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        match stage.step {
            0 => self.id.decode(buffer, stage),
            1 => self.amount.decode(buffer, stage),
            _ => Err(DecodeError::InternalError),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::byte32::tests::gen_bytes;
    use super::i32::tests::random_usize;
    use super::u256::tests::get_test_vector;
    use super::{new_decoder, Hash, Token, U256};
    use crate::buffer::Buffer;
    use crate::decode::Decoder;
    use crate::TempData;

    #[test]
    fn test_decode_token() {
        let u256_data = get_test_vector();
        let mut temp_data = TempData::new();

        for _ in 0..10 {
            let idx = random_usize(0, u256_data.len() - 1);
            let mut bytes = gen_bytes(32, 32);
            let hash = Hash::from_bytes(bytes.as_slice().try_into().unwrap());
            let u256_encoded = &u256_data[idx].0;
            let u256 = U256::from_encoded_bytes(u256_encoded);
            let token = Token {
                id: hash,
                amount: u256,
            };
            bytes.extend(u256_encoded);

            {
                let mut decoder = new_decoder::<Token>();
                let mut buffer = Buffer::new(&bytes, &mut temp_data).unwrap();
                assert_eq!(decoder.decode(&mut buffer), Ok(Some(&token)));
            }

            let mut length: usize = 0;
            let mut decoder = new_decoder::<Token>();

            while length < bytes.len() {
                let remain = bytes.len() - length;
                let size = random_usize(0, remain);
                let mut buffer =
                    Buffer::new(&bytes[length..(length + size)], &mut temp_data).unwrap();
                length += size;

                let result = decoder.decode(&mut buffer).unwrap();
                if length == bytes.len() {
                    assert_eq!(result, Some(&token));
                    assert!(decoder.stage.is_complete())
                } else {
                    assert_eq!(result, None);
                }
            }
        }
    }
}
