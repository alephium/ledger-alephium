use super::{Byte32, Hash, U16};
use crate::buffer::Buffer;
use crate::decode::*;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct P2MPKH {
    pub size: U16,
    pub m: U16,
}

impl Reset for P2MPKH {
    fn reset(&mut self) {
        self.size.reset();
        self.m.reset();
    }
}

impl P2MPKH {
    const MAX_KEY_NUM: usize = 4;
    // from: https://github.com/bitcoin/libbase58/blob/b1dd03fa8d1be4be076bb6152325c6b5cf64f678/base58.c#L155
    pub const BASE58_OUTPUT_SIZE: usize = 181;

    pub fn is_reviewable(&self) -> bool {
        (self.size.inner as usize) <= Self::MAX_KEY_NUM
    }
}

impl RawDecoder for P2MPKH {
    fn step_size(&self) -> u16 {
        3
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        match stage.step {
            0 => self.size.decode(buffer, stage),
            1 => {
                let total_length = (self.size.inner as usize) * Byte32::ENCODED_LENGTH;
                let mut index = stage.index;
                while !buffer.is_empty() && (index as usize) < total_length {
                    let byte = buffer.next_byte().unwrap();
                    buffer.write_byte_to_temp_data(byte);
                    index += 1;
                }
                if (index as usize) == total_length {
                    Ok(DecodeStage::COMPLETE)
                } else {
                    Ok(DecodeStage {
                        step: stage.step,
                        index,
                    })
                }
            }
            2 => self.m.decode(buffer, stage),
            _ => Err(DecodeError::InternalError),
        }
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum LockupScript {
    P2PKH(Hash),
    P2MPKH(PartialDecoder<P2MPKH>),
    P2SH(Hash),
    P2C(Hash),
    Unknown,
}

impl Reset for LockupScript {
    fn reset(&mut self) {
        *self = Self::Unknown;
    }
}

impl Default for LockupScript {
    fn default() -> Self {
        LockupScript::Unknown
    }
}

impl LockupScript {
    fn from_type(tpe: u8) -> Option<Self> {
        match tpe {
            0 => Some(LockupScript::P2PKH(Hash::default())),
            1 => Some(LockupScript::P2MPKH(PartialDecoder::default())),
            2 => Some(LockupScript::P2SH(Hash::default())),
            3 => Some(LockupScript::P2C(Hash::default())),
            _ => None,
        }
    }

    pub fn get_type(&self) -> u8 {
        match self {
            LockupScript::P2PKH(_) => 0,
            LockupScript::P2MPKH(_) => 1,
            LockupScript::P2SH(_) => 2,
            LockupScript::P2C(_) => 3,
            _ => 0xff, // dead branch
        }
    }
}

impl RawDecoder for LockupScript {
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
            LockupScript::Unknown => {
                let tpe = buffer.next_byte().unwrap();
                let result = LockupScript::from_type(tpe);
                if result.is_none() {
                    return Err(DecodeError::InvalidData);
                }
                *self = result.unwrap();
            }
            _ => (),
        };
        match self {
            LockupScript::P2PKH(hash) => hash.decode(buffer, stage),
            LockupScript::P2MPKH(hashes) => hashes.decode_children(buffer, stage),
            LockupScript::P2SH(hash) => hash.decode(buffer, stage),
            LockupScript::P2C(hash) => hash.decode(buffer, stage),
            LockupScript::Unknown => Err(DecodeError::InternalError),
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
    use crate::types::{Hash, LockupScript};
    use crate::TempData;
    use std::vec;

    #[test]
    fn test_decode_p2pkh() {
        for _ in 0..10 {
            let mut bytes = vec![0u8];
            let hash_bytes = gen_bytes(32, 32);
            bytes.extend(&hash_bytes);
            let lockup_script =
                LockupScript::P2PKH(Hash::from_bytes(hash_bytes.as_slice().try_into().unwrap()));
            let mut temp_data = TempData::new();

            {
                let mut buffer = Buffer::new(&bytes, &mut temp_data).unwrap();
                let mut decoder = new_decoder::<LockupScript>();
                let result = decoder.decode(&mut buffer).unwrap();
                assert_eq!(result, Some(&lockup_script));
            }

            let mut length: usize = 0;
            let mut decoder = new_decoder::<LockupScript>();

            while length < bytes.len() {
                let remain = bytes.len() - length;
                let size = random_usize(0, remain);
                let mut buffer =
                    Buffer::new(&bytes[length..(length + size)], &mut temp_data).unwrap();
                length += size;

                let result = decoder.decode(&mut buffer).unwrap();
                if length == bytes.len() {
                    assert_eq!(result, Some(&lockup_script));
                    assert!(decoder.stage.is_complete())
                } else {
                    assert_eq!(result, None);
                }
            }
        }
    }
}
