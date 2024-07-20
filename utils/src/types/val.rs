use crate::buffer::Writable;
use crate::decode::*;
use crate::types::*;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub enum Val {
    Bool(Bool),
    I256(I256),
    U256(U256),
    ByteVec(ByteString),
    Address(LockupScript),
    #[default]
    Unknown,
}

impl Reset for Val {
    fn reset(&mut self) {
        *self = Self::Unknown;
    }
}

impl Val {
    fn from_type(tpe: u8) -> Option<Self> {
        match tpe {
            0 => Some(Val::Bool(Bool::default())),
            1 => Some(Val::I256(I256::default())),
            2 => Some(Val::U256(U256::default())),
            3 => Some(Val::ByteVec(ByteString::default())),
            4 => Some(Val::Address(LockupScript::Unknown)),
            _ => None,
        }
    }
}

impl RawDecoder for Val {
    fn step_size(&self) -> u16 {
        match self {
            Val::Bool(v) => v.step_size(),
            Val::I256(v) => v.step_size(),
            Val::U256(v) => v.step_size(),
            Val::ByteVec(v) => v.step_size(),
            Val::Address(v) => v.step_size(),
            Val::Unknown => 1,
        }
    }

    fn decode<W: Writable>(
        &mut self,
        buffer: &mut crate::buffer::Buffer<'_, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if buffer.is_empty() {
            return Ok(DecodeStage { ..*stage });
        }
        if let Val::Unknown = self {
            let tpe = buffer.consume_byte().unwrap();
            let result = Val::from_type(tpe);
            if result.is_none() {
                return Err(DecodeError::InvalidData);
            }
            *self = result.unwrap();
        };

        match self {
            Val::Bool(v) => v.decode(buffer, stage),
            Val::I256(v) => v.decode(buffer, stage),
            Val::U256(v) => v.decode(buffer, stage),
            Val::ByteVec(v) => v.decode(buffer, stage),
            Val::Address(v) => v.decode(buffer, stage),
            Val::Unknown => Err(DecodeError::InternalError),
        }
    }
}
