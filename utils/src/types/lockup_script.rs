use super::{Byte32, Hash, U16};
use crate::buffer::{Buffer, Writable};
use crate::types::{Byte, Checksum, Checksumable, Checksummed, PublicKeyLike};
use crate::{decode::*, djb_hash};

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
pub struct P2PK {
    pub key: Checksummed<PublicKeyLike>,
    pub group: Byte,
}

impl Reset for P2PK {
    fn reset(&mut self) {
        self.key.reset();
        self.group.reset();
    }
}

impl RawDecoder for P2PK {
    fn step_size(&self) -> u16 {
        self.key.step_size() + 1
    }

    fn decode<W: Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        match stage.step {
            step if step < self.key.step_size() => self.key.decode(buffer, stage),
            step if step < self.step_size() => self.group.decode(buffer, stage),
            _ => Err(DecodeError::InternalError),
        }
    }
}

impl Checksumable for Hash {
    fn calc_checksum(&self) -> Checksum {
        let hash: [u8; 4] = djb_hash(&self.0).to_be_bytes();
        Checksum(hash)
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct P2HMPK {
    pub hash: Checksummed<Hash>,
    pub group: Byte,
}

impl Reset for P2HMPK {
    fn reset(&mut self) {
        self.hash.reset();
        self.group.reset();
    }
}

impl RawDecoder for P2HMPK {
    fn step_size(&self) -> u16 {
        self.hash.step_size() + 1
    }

    fn decode<W: Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        match stage.step {
            step if step < self.hash.step_size() => self.hash.decode(buffer, stage),
            step if step < self.step_size() => self.group.decode(buffer, stage),
            _ => Err(DecodeError::InternalError),
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
    P2PK(StreamingDecoder<P2PK>),
    P2HMPK(StreamingDecoder<P2HMPK>),
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
            4 => Some(LockupScript::P2PK(StreamingDecoder::default())),
            5 => Some(LockupScript::P2HMPK(StreamingDecoder::default())),
            _ => None,
        }
    }

    pub fn get_type(&self) -> u8 {
        match self {
            LockupScript::P2PKH(_) => 0,
            LockupScript::P2MPKH(_) => 1,
            LockupScript::P2SH(_) => 2,
            LockupScript::P2C(_) => 3,
            LockupScript::P2PK(_) => 4,
            LockupScript::P2HMPK(_) => 5,
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
            LockupScript::P2PK(p2pk) => p2pk.decode_children(buffer, stage),
            LockupScript::P2HMPK(p2hmpk) => p2hmpk.decode_children(buffer, stage),
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
    use crate::types::test_utils::{test_decode, test_decode1};
    use crate::types::u256::tests::hex_to_bytes;
    use crate::types::{Hash, LockupScript, PublicKeyLike, SecP256R1PubKey};
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
        let data = bytes[1..].to_vec().clone();
        test_decode::<LockupScript>(01u8, data, None, Some(&bytes));
    }

    #[test]
    fn test_decode_p2pk() {
        let data = hex_to_bytes(
            "036394a7ef0dee6b4a89fe1be67f89fe7bd87c69370c80f7c1773fc7a556ec0fbd02ca39a41801",
        )
        .unwrap();
        let key_bytes =
            hex_to_bytes("6394a7ef0dee6b4a89fe1be67f89fe7bd87c69370c80f7c1773fc7a556ec0fbd02")
                .unwrap();
        let key =
            PublicKeyLike::WebAuthn(SecP256R1PubKey::from_bytes(key_bytes.try_into().unwrap()));
        let check = |result: Option<&LockupScript>| match result.unwrap() {
            LockupScript::P2PK(inner) => {
                assert!(inner.stage.is_complete());
                assert_eq!(inner.inner.key.value, key);
                assert_eq!(inner.inner.group.0, 1);
            }
            _ => assert!(false),
        };
        test_decode1(4u8, data, &check, None);
    }

    #[test]
    fn test_decode_p2hmpk() {
        let data = hex_to_bytes(
            "d86790cb655097d8e6f357af86aab2ee1c21ea1462a64d323938a3fc5824d1433f359ff402",
        )
        .unwrap();
        let hash = Hash::from_bytes(
            hex_to_bytes("d86790cb655097d8e6f357af86aab2ee1c21ea1462a64d323938a3fc5824d143")
                .unwrap()
                .as_slice()
                .try_into()
                .unwrap(),
        );

        let check = |result: Option<&LockupScript>| match result.unwrap() {
            LockupScript::P2HMPK(inner) => {
                assert!(inner.stage.is_complete());
                assert_eq!(inner.inner.hash.value, hash);
                assert_eq!(inner.inner.group.0, 2);
            }
            _ => assert!(false),
        };
        test_decode1(5u8, data, &check, None);
    }
}
