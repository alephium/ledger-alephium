use crate::buffer::{Buffer, Writable};
use crate::decode::*;
use crate::types::compact_integer::*;

#[cfg_attr(test, derive(Debug))]
#[derive(Default)]
pub struct U16 {
    pub inner: u16,
    first_byte: u8,
}

impl Reset for U16 {
    fn reset(&mut self) {
        self.inner = 0;
        self.first_byte = 0;
    }
}

impl U16 {
    #[inline]
    pub fn get_length(&self) -> usize {
        decode_length(self.first_byte)
    }

    pub fn from(num: u16) -> Self {
        Self {
            inner: num,
            first_byte: 0,
        }
    }
}

impl PartialEq for U16 {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl RawDecoder for U16 {
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
        if stage.index == 0 {
            self.first_byte = buffer.consume_byte().unwrap();
        }
        let length = self.get_length();
        if length > 4 {
            return Err(DecodeError::InvalidSize);
        }

        let mut index = if stage.index == 0 {
            self.inner = (((self.first_byte as u32) & MASK_MODE) << ((length - 1) * 8)) as u16;
            1
        } else {
            stage.index as usize
        };

        while !buffer.is_empty() && index < length {
            let byte = buffer.consume_byte().unwrap() as u32;
            self.inner |= ((byte & 0xff) << ((length - index - 1) * 8)) as u16;
            index += 1;
        }
        if index == length {
            Ok(DecodeStage::COMPLETE)
        } else {
            Ok(DecodeStage {
                step: stage.step,
                index: index as u16,
            })
        }
    }
}

#[cfg(test)]
pub mod tests {
    extern crate std;

    use crate::TempData;

    use super::*;
    use std::vec;

    #[test]
    fn test_decode_u16() {
        let items = [
            (vec![0x80u8, 0x00, 0xff, 0xff], u16::MAX),
            (vec![0x80u8, 0x00, 0xff, 0x00], 0xff00),
            (vec![0x40u8, 0xff], 0xff),
            (vec![0x0fu8], 0x0f),
            (vec![0x00u8], 0x00),
        ];

        let mut temp_data = TempData::new();
        for (bytes, num) in items {
            let mut buffer = Buffer::new(&bytes, &mut temp_data);
            let mut decoder = new_decoder::<U16>();
            let result = decoder.decode(&mut buffer).unwrap().unwrap();
            assert_eq!(result.inner, num);
        }
    }
}
