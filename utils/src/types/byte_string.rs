use crate::buffer::Buffer;
use crate::decode::*;

use super::I32;

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct ByteString {
    length: I32,
    current_index: i32,
}

impl Reset for ByteString {
    fn reset(&mut self) {
        self.length.reset();
        self.current_index = -1;
    }
}

impl Default for ByteString {
    fn default() -> Self {
        Self {
            length: I32::default(),
            current_index: -1,
        }
    }
}

impl ByteString {
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            length: I32::from(0),
            current_index: 0,
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.length.inner as usize
    }

    #[inline]
    fn total_size_decoded(&self) -> bool {
        self.current_index >= 0
    }

    #[cfg(test)]
    #[inline]
    fn is_complete(&self) -> bool {
        (self.current_index as usize) == self.size()
    }
}

impl RawDecoder for ByteString {
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if !self.total_size_decoded() {
            let result = self.length.decode(buffer, stage)?;
            if !result.is_complete() {
                return Ok(DecodeStage { ..*stage });
            }
            self.current_index = 0;
        }

        while (self.current_index as usize) < self.size() && !buffer.is_empty() {
            let _ = buffer.next_byte().unwrap();
            self.current_index += 1;
        }

        if (self.current_index as usize) == self.size() {
            Ok(DecodeStage::COMPLETE)
        } else {
            Ok(DecodeStage { ..*stage })
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use crate::TempData;

    use super::*;
    use std::vec;

    #[test]
    fn test_decode_empty_byte_string() {
        let mut temp_data = TempData::new();
        let bytes = vec![0u8];
        let mut decoder = new_decoder::<ByteString>();
        let mut buffer = Buffer::new(&bytes, &mut temp_data).unwrap();

        let result = decoder.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(result.size(), 0);
        assert!(result.is_complete());
    }

    #[test]
    fn test_decode_byte_string() {
        let mut temp_data = TempData::new();
        let bytes = vec![4u8, 0, 1, 2, 3];
        let mut decoder = new_decoder::<ByteString>();
        let mut buffer = Buffer::new(&bytes, &mut temp_data).unwrap();

        let result = decoder.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(result.size(), 4);
        assert!(result.is_complete());
    }
}
