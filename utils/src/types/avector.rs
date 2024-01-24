use core::cmp;

use super::I32;
use crate::buffer::Buffer;
use crate::decode::*;

#[cfg_attr(test, derive(Debug))]
pub struct AVector<T> {
    current_item: PartialDecoder<T>,
    current_item_complete: bool,
    pub current_index: isize,
    total_size: I32,
}

impl<T> AVector<T> {
    pub fn get_current_item(&self) -> Option<&T> {
        if self.current_item_complete {
            Some(&self.current_item.inner)
        } else {
            None
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.total_size.inner as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }

    #[inline]
    fn total_size_decoded(&self) -> bool {
        self.current_index >= 0
    }

    pub fn is_complete(&self) -> bool {
        if !self.total_size_decoded() {
            return false;
        }
        if self.is_empty() {
            return true;
        }
        return ((self.current_index as usize) == (self.size() - 1)) && self.current_item_complete;
    }
}

impl<T: Default + RawDecoder> Default for AVector<T> {
    fn default() -> Self {
        AVector {
            current_item: new_decoder::<T>(),
            current_item_complete: false,
            current_index: -1,
            total_size: I32::default(),
        }
    }
}

impl<T: Default + RawDecoder> RawDecoder for AVector<T> {
    fn step_size(&self) -> usize {
        if self.total_size_decoded() {
            cmp::max(self.size(), 1)
        } else {
            1
        }
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if !self.total_size_decoded() {
            let result = self.total_size.decode(buffer, stage)?;
            if !result.is_complete() {
                return Ok(DecodeStage { ..*stage });
            }
            self.current_index = 0;
            if self.size() == 0 {
                return Ok(DecodeStage::COMPLETE);
            }
        }

        if self.current_item_complete {
            self.current_item_complete = false;
            self.current_item.reset();
            self.current_index += 1;
        }

        let result = self.current_item.decode(buffer)?;
        if result.is_none() {
            return Ok(DecodeStage { ..*stage });
        }

        self.current_item_complete = true;
        return Ok(DecodeStage::COMPLETE);
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::AVector;
    use crate::buffer::Buffer;
    use crate::decode::{new_decoder, Decoder};
    use crate::types::byte32::tests::gen_bytes;
    use crate::types::i32::tests::random_usize;
    use crate::types::{Hash, I32, U256};
    use std::vec;
    use std::vec::Vec;

    #[test]
    fn test_decode_empty_avector() {
        let empty_avector_encoded = vec![0u8];
        let mut buffer0 = Buffer::new(&empty_avector_encoded).unwrap();
        let mut decoder0 = new_decoder::<AVector<Hash>>();
        let result0 = decoder0.decode(&mut buffer0).unwrap().unwrap();
        assert!(result0.get_current_item().is_none());
        assert!(result0.is_complete());

        let mut buffer1 = Buffer::new(&empty_avector_encoded).unwrap();
        let mut decoder1 = new_decoder::<AVector<U256>>();
        let result1 = decoder1.decode(&mut buffer1).unwrap().unwrap();
        assert!(result1.get_current_item().is_none());
        assert!(result1.is_complete());
    }

    #[test]
    fn test_decode_avector() {
        let max_size: usize = 0x1f;
        for _ in 0..10 {
            let size = random_usize(1, max_size);
            let mut hashes: Vec<Hash> = Vec::with_capacity(size);
            let mut bytes = vec![size as u8];
            for _ in 0..size {
                let hash_bytes = gen_bytes(32, 32);
                hashes.push(Hash::from_bytes(hash_bytes.as_slice().try_into().unwrap()));
                bytes.extend(&hash_bytes);
            }

            if bytes.len() <= (u8::MAX as usize) {
                let mut buffer = Buffer::new(&bytes).unwrap();
                let mut decoder = new_decoder::<AVector<Hash>>();
                let result = decoder.decode(&mut buffer).unwrap().unwrap();
                assert_eq!(result.total_size, I32::from(size as i32));
                assert_eq!(result.get_current_item(), hashes.last());
                assert_eq!(result.current_index as usize, size - 1);
            }

            let mut length: usize = 0;
            let mut decoder = new_decoder::<AVector<Hash>>();

            while length < bytes.len() {
                let size = if length == 0 { 33 } else { 32 };
                let mut buffer = Buffer::new(&bytes[length..(length + size)]).unwrap();
                length += size;

                let result = decoder.decode(&mut buffer).unwrap();
                if length == bytes.len() {
                    assert!(result.is_some());
                    assert!(decoder.stage.is_complete());
                } else {
                    let index = length / 32;
                    assert!(result.is_none());
                    assert_eq!(decoder.stage.step, index);
                    assert_eq!(decoder.inner.get_current_item(), Some(&hashes[index - 1]));
                }
            }
        }
    }
}
