use crate::buffer::{Buffer, Writable};
use crate::decode::*;
use crate::types::compact_integer::*;

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct BigInt {
    pub bytes: [u8; 33],
}

impl Reset for BigInt {
    fn reset(&mut self) {
        self.bytes = [0; 33];
    }
}

impl Default for BigInt {
    fn default() -> Self {
        BigInt { bytes: [0; 33] }
    }
}

impl PartialEq for BigInt {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl BigInt {
    #[cfg(test)]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert!(bytes.len() == 33);
        let mut bs = [0u8; 33];
        bs.copy_from_slice(bytes);
        Self { bytes: bs }
    }

    #[inline]
    pub fn get_length(&self) -> usize {
        decode_length(self.bytes[0])
    }

    #[inline]
    pub fn is_fixed_size(&self) -> bool {
        is_fixed_size(self.bytes[0])
    }
}

impl RawDecoder for BigInt {
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
        let from_index = if stage.index == 0 {
            self.bytes[0] = buffer.consume_byte().unwrap();
            1
        } else {
            stage.index
        };
        let length = self.get_length();
        let mut idx = 0;
        while !buffer.is_empty() && idx < (length - (from_index as usize)) {
            self.bytes[(from_index as usize) + idx] = buffer.consume_byte().unwrap();
            idx += 1;
        }
        let new_index = (from_index as usize) + idx;
        if new_index == length {
            Ok(DecodeStage::COMPLETE)
        } else {
            Ok(DecodeStage {
                step: stage.step,
                index: new_index as u16,
            })
        }
    }
}
