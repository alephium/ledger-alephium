use super::*;
use crate::buffer::{Buffer, Writable};
use crate::decode::*;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct Method {
    is_public: Byte,
    asset_modifier: Byte,
    args_length: U16,
    locals_length: U16,
    return_length: U16,
    instrs: AVector<Instr>,
}

impl Reset for Method {
    fn reset(&mut self) {
        self.is_public.reset();
        self.asset_modifier.reset();
        self.args_length.reset();
        self.locals_length.reset();
        self.return_length.reset();
        self.instrs.reset();
    }
}

impl RawDecoder for Method {
    fn step_size(&self) -> u16 {
        5 + self.instrs.step_size()
    }

    fn decode<W: Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
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
