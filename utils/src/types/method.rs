use super::*;
use crate::buffer::Buffer;
use crate::decode::*;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct Method {
    is_public: Byte,
    asset_modifier: Byte,
    args_length: I32,
    locals_length: I32,
    return_length: I32,
    instrs: AVector<Instr>,
}

impl RawDecoder for Method {
    fn step_size(&self) -> u16 {
        5 + self.instrs.step_size()
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        match stage.step {
            0 => self.is_public.decode(buffer, stage),
            1 => self.asset_modifier.decode(buffer, stage),
            2 => self.args_length.decode(buffer, stage),
            3 => self.locals_length.decode(buffer, stage),
            4 => self.return_length.decode(buffer, stage),
            step if step < self.step_size() => self.instrs.decode(buffer, stage),
            _ => Err(DecodeError::InternalError),
        }
    }
}
