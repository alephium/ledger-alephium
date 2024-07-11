use core::cmp;

use super::U16;
use crate::buffer::{Buffer, Writable};
use crate::decode::*;

#[cfg_attr(test, derive(Debug))]
pub struct AVector<T> {
    current_item: PartialDecoder<T>,
    total_size: U16,
    pub current_index: i16,
}

impl<T: Reset> Reset for AVector<T> {
    fn reset(&mut self) {
        self.current_item.reset();
        self.total_size.reset();
        self.current_index = -1;
    }
}

#[cfg(test)]
impl<T: PartialEq> PartialEq for AVector<T> {
    fn eq(&self, other: &Self) -> bool {
        self.current_item.inner == other.current_item.inner
    }
}

#[cfg(test)]
impl<T: Default + RawDecoder> AVector<T> {
    pub fn from_item(value: T) -> Self {
        AVector {
            current_item: PartialDecoder {
                inner: value,
                stage: DecodeStage::default(),
            },
            ..AVector::default()
        }
    }
}

impl<T> AVector<T> {
    pub fn get_current_item(&self) -> Option<&T> {
        if self.current_item.stage.is_complete() {
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
        return ((self.current_index as usize) == (self.size() - 1))
            && self.current_item.stage.is_complete();
    }
}

impl<T: Default + RawDecoder> Default for AVector<T> {
    fn default() -> Self {
        AVector {
            current_item: new_decoder::<T>(),
            current_index: -1,
            total_size: U16::default(),
        }
    }
}

impl<T: Reset + RawDecoder> RawDecoder for AVector<T> {
    fn step_size(&self) -> u16 {
        if self.total_size_decoded() {
            cmp::max(self.size() as u16, 1)
        } else {
            1
        }
    }

    fn decode<'a, W: Writable>(
        &mut self,
        buffer: &mut Buffer<'a, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if !self.total_size_decoded() {
            let result = self.total_size.decode(buffer, stage)?;
            if !result.is_complete() {
                return Ok(result);
            }
            self.current_index = 0;
            if self.size() == 0 {
                return Ok(DecodeStage::COMPLETE);
            }
        }

        if self.current_item.stage.is_complete() {
            self.current_item.reset();
            self.current_index += 1;
        }

        let result = self.current_item.decode(buffer)?;
        if result.is_none() {
            return Ok(DecodeStage { ..*stage });
        }

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
    use crate::types::{Hash, U16, U256};
    use crate::TempData;
    use std::vec;
    use std::vec::Vec;

    #[test]
    fn test_decode_empty_avector() {
        let mut temp_data = TempData::new();
        let empty_avector_encoded = vec![0u8];
        let mut buffer0 = Buffer::new(&empty_avector_encoded, &mut temp_data).unwrap();
        let mut decoder0 = new_decoder::<AVector<Hash>>();
        let result0 = decoder0.decode(&mut buffer0).unwrap().unwrap();
        assert!(result0.get_current_item().is_none());
        assert!(result0.is_complete());

        let mut buffer1 = Buffer::new(&empty_avector_encoded, &mut temp_data).unwrap();
        let mut decoder1 = new_decoder::<AVector<U256>>();
        let result1 = decoder1.decode(&mut buffer1).unwrap().unwrap();
        assert!(result1.get_current_item().is_none());
        assert!(result1.is_complete());
    }

    pub fn encode_size(size: usize) -> Vec<u8> {
        if size < 0x20 {
            return vec![(size & 0xff) as u8];
        }
        if size < (0x20 << 8) {
            return vec![(((size >> 8) & 0xff) as u8) + 0x40, (size & 0xff) as u8];
        }
        return vec![
            (((size >> 24) & 0xff) as u8) + 0x80,
            ((size >> 16) & 0xff) as u8,
            ((size >> 8) & 0xff) as u8,
            (size & 0xff) as u8,
        ];
    }

    #[test]
    fn test_decode_avector() {
        let max_size: i16 = i16::MAX - 1;
        let mut temp_data = TempData::new();
        for _ in 0..10 {
            let size = random_usize(1, max_size as usize);
            let mut hashes: Vec<Hash> = Vec::with_capacity(size);
            let mut bytes = encode_size(size);
            let prefix_length = bytes.len();
            for _ in 0..size {
                let hash_bytes = gen_bytes(32, 32);
                hashes.push(Hash::from_bytes(hash_bytes.as_slice().try_into().unwrap()));
                bytes.extend(&hash_bytes);
            }

            if bytes.len() <= (u8::MAX as usize) {
                let mut buffer = Buffer::new(&bytes, &mut temp_data).unwrap();
                let mut decoder = new_decoder::<AVector<Hash>>();
                let result = decoder.decode(&mut buffer).unwrap().unwrap();
                assert_eq!(result.total_size, U16::from(size as u16));
                assert_eq!(result.get_current_item(), hashes.last());
                assert_eq!(result.current_index as usize, size - 1);
            }

            let mut length: usize = 0;
            let mut decoder = new_decoder::<AVector<Hash>>();

            while length < bytes.len() {
                let size = if length == 0 { 32 + prefix_length } else { 32 };
                let mut buffer =
                    Buffer::new(&bytes[length..(length + size)], &mut temp_data).unwrap();
                length += size;

                let result = decoder.decode(&mut buffer).unwrap();
                if length == bytes.len() {
                    assert!(result.is_some());
                    assert!(decoder.stage.is_complete());
                } else {
                    let index = length / 32;
                    assert!(result.is_none());
                    assert_eq!(decoder.stage.step as usize, index);
                    assert_eq!(decoder.inner.get_current_item(), Some(&hashes[index - 1]));
                }
            }
        }
    }
}
