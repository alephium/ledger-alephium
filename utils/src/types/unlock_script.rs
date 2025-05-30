use super::*;
use crate::buffer::{Buffer, Writable};
use crate::decode::*;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct PublicKeyWithIndex {
    public_key: SecP256K1PubKey,
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

    fn decode<W: Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
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

    fn decode<W: Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
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
#[derive(Default)]
pub enum UnlockScript {
    P2PKH(SecP256K1PubKey),
    P2MPKH(StreamingDecoder<AVector<PublicKeyWithIndex>>),
    P2SH(StreamingDecoder<P2SH>),
    SameAsPrevious,
    PoLW(SecP256K1PubKey),
    P2PK,
    #[default]
    Unknown,
}

impl Reset for UnlockScript {
    fn reset(&mut self) {
        *self = Self::Unknown;
    }
}

impl UnlockScript {
    fn from_type(tpe: u8) -> Option<Self> {
        match tpe {
            0 => Some(UnlockScript::P2PKH(SecP256K1PubKey::default())),
            1 => Some(UnlockScript::P2MPKH(StreamingDecoder::default())),
            2 => Some(UnlockScript::P2SH(StreamingDecoder::default())),
            3 => Some(UnlockScript::SameAsPrevious),
            4 => Some(UnlockScript::PoLW(SecP256K1PubKey::default())),
            5 => Some(UnlockScript::P2PK),
            _ => None,
        }
    }
}

impl RawDecoder for UnlockScript {
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
        if let UnlockScript::Unknown = self {
            let tpe = buffer.consume_byte().unwrap();
            let result = UnlockScript::from_type(tpe);
            if result.is_none() {
                return Err(DecodeError::InvalidData);
            }
            *self = result.unwrap();
        };
        match self {
            UnlockScript::P2PKH(public_key) => public_key.decode(buffer, stage),
            UnlockScript::P2MPKH(keys) => keys.decode_children(buffer, stage),
            UnlockScript::P2SH(script) => script.decode_children(buffer, stage),
            UnlockScript::SameAsPrevious => Ok(DecodeStage::COMPLETE),
            UnlockScript::PoLW(public_key) => public_key.decode(buffer, stage),
            UnlockScript::P2PK => Ok(DecodeStage::COMPLETE),
            UnlockScript::Unknown => Err(DecodeError::InternalError),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::u256::tests::hex_to_bytes;
    use crate::types::byte32::tests::gen_bytes;
    use crate::types::test_utils::test_decode;
    use crate::types::{SecP256K1PubKey, UnlockScript};
    use std::vec;

    #[test]
    fn test_decode_p2pkh() {
        let public_key = gen_bytes(33, 33);
        let unlock_script = UnlockScript::P2PKH(SecP256K1PubKey::from_bytes(
            public_key.clone().try_into().unwrap(),
        ));
        test_decode(0u8, public_key, Some(&unlock_script), None)
    }

    #[test]
    fn test_decode_p2sh() {
        let bytecode = hex_to_bytes("010100000000045814402000000000000000000000000000000000000000000000000000000000000000008685").unwrap();
        let data = hex_to_bytes("01010000000004581440200000000000000000000000000000000000000000000000000000000000000000868500").unwrap();
        test_decode::<UnlockScript>(2u8, data, None, Some(&bytecode));
    }

    #[test]
    fn test_decode_polw() {
        let public_key = gen_bytes(33, 33);
        let unlock_script = UnlockScript::PoLW(SecP256K1PubKey::from_bytes(
            public_key.clone().try_into().unwrap(),
        ));
        test_decode(4u8, public_key, Some(&unlock_script), None)
    }

    #[test]
    fn test_decode_p2pk() {
        test_decode(5u8, vec![], Some(&UnlockScript::P2PK), None)
    }
}
