use super::{Byte32, Hash, U16};
use crate::buffer::{Buffer, Writable};
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

impl RawDecoder for P2MPKH {
    fn step_size(&self) -> u16 {
        3
    }

    fn decode<W: Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        let from_index = buffer.get_index();
        let result = match stage.step {
            0 => {
                if stage.index == 0 {
                    buffer.write_bytes_to_temp_data(&[1u8])?; // write prefix
                }
                self.size.decode(buffer, stage)
            }
            1 => {
                let total_length = (self.size.inner as usize) * Byte32::ENCODED_LENGTH;
                let mut index = stage.index;
                while !buffer.is_empty() && (index as usize) < total_length {
                    let _ = buffer.consume_byte().unwrap();
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
        };
        match result {
            Err(err) => Err(err),
            Ok(value) => {
                let to_index = buffer.get_index();
                buffer.write_bytes_to_temp_data(buffer.get_range(from_index, to_index))?;
                Ok(value)
            }
        }
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub enum LockupScript {
    P2PKH(Hash),
    P2MPKH(StreamingDecoder<P2MPKH>),
    P2SH(Hash),
    P2C(Hash),
    #[default]
    Unknown,
}

impl Reset for LockupScript {
    fn reset(&mut self) {
        *self = Self::Unknown;
    }
}

impl LockupScript {
    fn from_type(tpe: u8) -> Option<Self> {
        match tpe {
            0 => Some(LockupScript::P2PKH(Hash::default())),
            1 => Some(LockupScript::P2MPKH(StreamingDecoder::default())),
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

    fn decode<W: Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if buffer.is_empty() {
            return Ok(DecodeStage { ..*stage });
        }
        if let LockupScript::Unknown = self {
            let tpe = buffer.consume_byte().unwrap();
            let result = LockupScript::from_type(tpe);
            if result.is_none() {
                return Err(DecodeError::InvalidData);
            }
            *self = result.unwrap();
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
    use crate::types::u256::tests::hex_to_bytes;
    use crate::types::{Hash, LockupScript};
    use crate::TempData;
    use std::vec;

    fn test(prefix: u8, ctor: fn(Hash) -> LockupScript) {
        for _ in 0..10 {
            let mut bytes = vec![prefix];
            let hash_bytes = gen_bytes(32, 32);
            bytes.extend(&hash_bytes);
            let lockup_script = ctor(Hash::from_bytes(hash_bytes.as_slice().try_into().unwrap()));
            let mut temp_data = TempData::new();

            {
                let mut buffer = Buffer::new(&bytes, &mut temp_data);
                let mut decoder = new_decoder::<LockupScript>();
                let result = decoder.decode(&mut buffer).unwrap();
                assert_eq!(result, Some(&lockup_script));
            }

            let mut length: usize = 0;
            let mut decoder = new_decoder::<LockupScript>();

            while length < bytes.len() {
                let remain = bytes.len() - length;
                let size = random_usize(0, remain);
                let mut buffer = Buffer::new(&bytes[length..(length + size)], &mut temp_data);
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

    #[test]
    fn test_decode_p2pkh() {
        test(0, |hash| LockupScript::P2PKH(hash))
    }

    #[test]
    fn test_decode_p2sh() {
        test(2, |hash| LockupScript::P2SH(hash))
    }

    #[test]
    fn test_decode_p2c() {
        test(3, |hash| LockupScript::P2C(hash))
    }

    #[test]
    fn test_decode_p2mpkh() {
        let bytes = hex_to_bytes("0103a3cd757be03c7dac8d48bf79e2a7d6e735e018a9c054b99138c7b29738c437ecef51c98556924afa1cd1a8026c3d2d33ee1d491e1fe77c73a75a2d0129f061951dd2aa371711d1faea1c96d395f08eb94de1f388993e8be3f4609dc327ab513a02").unwrap();
        {
            let mut temp_data = TempData::new();
            let mut buffer = Buffer::new(&bytes, &mut temp_data);
            let mut decoder = new_decoder::<LockupScript>();
            let result = decoder.decode(&mut buffer).unwrap();
            assert!(result.is_some());
            assert!(decoder.stage.is_complete());
            assert_eq!(temp_data.get(), &bytes);
        }

        let mut temp_data = TempData::new();
        let mut length: usize = 0;
        let mut decoder = new_decoder::<LockupScript>();

        while length < bytes.len() {
            let remain = bytes.len() - length;
            let size = random_usize(0, remain);
            let mut buffer = Buffer::new(&bytes[length..(length + size)], &mut temp_data);
            length += size;

            let result = decoder.decode(&mut buffer).unwrap();
            if length == bytes.len() {
                assert!(result.is_some());
                assert!(decoder.stage.is_complete());
                assert_eq!(temp_data.get(), &bytes);
            } else {
                assert_eq!(result, None);
            }
        }
    }
}
