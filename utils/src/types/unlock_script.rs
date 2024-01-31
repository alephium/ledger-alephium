use super::*;
use crate::buffer::Buffer;
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

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
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
pub enum UnlockScript {
    P2PKH(PublicKey),
    P2MPKH(PartialDecoder<AVector<PublicKeyWithIndex>>),
    P2SH(PartialDecoder<(Script, AVector<Val>)>),
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
            1 => Some(UnlockScript::P2MPKH(PartialDecoder::default())),
            2 => Some(UnlockScript::P2SH(PartialDecoder::default())),
            3 => Some(UnlockScript::SameAsPrevious),
            _ => None,
        }
    }
}

impl RawDecoder for UnlockScript {
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if buffer.is_empty() {
            return Ok(DecodeStage { ..*stage });
        }
        match self {
            UnlockScript::Unknown => {
                let tpe = buffer.next_byte().unwrap();
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
                let mut buffer = Buffer::new(&bytes, &mut temp_data).unwrap();
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
                    Buffer::new(&bytes[length..(length + size)], &mut temp_data).unwrap();
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
}
