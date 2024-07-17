use super::*;
use crate::buffer::{Buffer, Writable};
use crate::decode::*;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct PublicKeyWithIndex {
    public_key: PublicKey,
    index: U16,
}

impl Reset for PublicKeyWithIndex {
    fn reset(&mut self) {
        self.public_key.reset();
        self.index.reset();
    }
}

impl RawDecoder for PublicKeyWithIndex {
    fn step_size(&self) -> u16 {
        2
    }

    fn decode<'a, W: Writable>(
        &mut self,
        buffer: &mut Buffer<'a, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        match stage.step {
            0 => self.public_key.decode(buffer, stage),
            1 => self.index.decode(buffer, stage),
            _ => Err(DecodeError::InternalError),
        }
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct P2SH(Script, AVector<Val>);

impl RawDecoder for P2SH {
    fn step_size(&self) -> u16 {
        self.0.step_size() + self.0.step_size()
    }

    fn decode<'a, W: Writable>(
        &mut self,
        buffer: &mut Buffer<'a, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        match stage.step {
            step if step < self.0.step_size() => {
                let from_index = buffer.get_index();
                let result = self.0.decode(buffer, stage);
                let to_index = buffer.get_index();
                let bytes = buffer.get_range(from_index, to_index);
                buffer.write_bytes_to_temp_data(bytes)?;
                result
            }
            _ => self.1.decode(buffer, stage),
        }
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum UnlockScript {
    P2PKH(PublicKey),
    P2MPKH(StreamingDecoder<AVector<PublicKeyWithIndex>>),
    P2SH(StreamingDecoder<P2SH>),
    SameAsPrevious,
    Unknown,
}

impl Reset for UnlockScript {
    fn reset(&mut self) {
        *self = Self::Unknown;
    }
}

impl Default for UnlockScript {
    fn default() -> Self {
        UnlockScript::Unknown
    }
}

impl UnlockScript {
    fn from_type(tpe: u8) -> Option<Self> {
        match tpe {
            0 => Some(UnlockScript::P2PKH(PublicKey::default())),
            1 => Some(UnlockScript::P2MPKH(StreamingDecoder::default())),
            2 => Some(UnlockScript::P2SH(StreamingDecoder::default())),
            3 => Some(UnlockScript::SameAsPrevious),
            _ => None,
        }
    }
}

impl RawDecoder for UnlockScript {
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<'a, W: Writable>(
        &mut self,
        buffer: &mut Buffer<'a, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if buffer.is_empty() {
            return Ok(DecodeStage { ..*stage });
        }
        match self {
            UnlockScript::Unknown => {
                let tpe = buffer.consume_byte().unwrap();
                let result = UnlockScript::from_type(tpe);
                if result.is_none() {
                    return Err(DecodeError::InvalidData);
                }
                *self = result.unwrap();
            }
            _ => (),
        };
        match self {
            UnlockScript::P2PKH(public_key) => public_key.decode(buffer, stage),
            UnlockScript::P2MPKH(keys) => keys.decode_children(buffer, stage),
            UnlockScript::P2SH(script) => script.decode_children(buffer, stage),
            UnlockScript::SameAsPrevious => Ok(DecodeStage::COMPLETE),
            UnlockScript::Unknown => Err(DecodeError::InternalError),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use crate::buffer::Buffer;
    use crate::decode::{new_decoder, Decoder};
    use crate::types::byte32::tests::gen_bytes;
    use crate::types::i32::tests::random_usize;
    use crate::types::{PublicKey, UnlockScript};
    use crate::TempData;
    use std::vec;

    use super::u256::tests::hex_to_bytes;

    #[test]
    fn test_decode_p2pkh() {
        let mut temp_data = TempData::new();
        for _ in 0..10 {
            let mut bytes = vec![0u8];
            let hash_bytes = gen_bytes(33, 33);
            bytes.extend(&hash_bytes);
            let unlock_script = UnlockScript::P2PKH(PublicKey::from_bytes(
                hash_bytes.as_slice().try_into().unwrap(),
            ));

            {
                let mut buffer = Buffer::new(&bytes, &mut temp_data);
                let mut decoder = new_decoder::<UnlockScript>();
                let result = decoder.decode(&mut buffer).unwrap();
                assert_eq!(result, Some(&unlock_script));
            }

            let mut length: usize = 0;
            let mut decoder = new_decoder::<UnlockScript>();

            while length < bytes.len() {
                let remain = bytes.len() - length;
                let size = random_usize(0, remain);
                let mut buffer =
                    Buffer::new(&bytes[length..(length + size)], &mut temp_data);
                length += size;

                let result = decoder.decode(&mut buffer).unwrap();
                if length == bytes.len() {
                    assert_eq!(result, Some(&unlock_script));
                    assert!(decoder.stage.is_complete())
                } else {
                    assert_eq!(result, None);
                }
            }
        }
    }

    #[test]
    fn test_decode_p2sh() {
        let mut temp_data = TempData::new();
        let bytecode = hex_to_bytes("010100000000045814402000000000000000000000000000000000000000000000000000000000000000008685").unwrap();
        let bytes = hex_to_bytes("0201010000000004581440200000000000000000000000000000000000000000000000000000000000000000868500").unwrap();

        let mut length: usize = 0;
        let mut decoder = new_decoder::<UnlockScript>();

        while length < bytes.len() {
            let remain = bytes.len() - length;
            let size = random_usize(0, remain);
            let mut buffer = Buffer::new(&bytes[length..(length + size)], &mut temp_data);
            length += size;

            let result = decoder.decode(&mut buffer).unwrap();
            if length == bytes.len() {
                assert_eq!(temp_data.get(), &bytecode);
                assert!(decoder.stage.is_complete());
            } else {
                assert_eq!(result, None);
            }
        }
    }
}
