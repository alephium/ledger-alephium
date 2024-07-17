use crate::decode::{DecodeError, DecodeResult};

pub trait Writable {
    fn write(&mut self, bytes: &[u8]) -> bool;
}

pub struct Buffer<'a, W> {
    index: usize,
    data: &'a [u8],
    temp_data: *mut W,
}

impl<'a, W> Buffer<'a, W> {
    pub fn new(data: &'a [u8], temp_data: *mut W) -> Buffer<'a, W> {
        Buffer {
            index: 0,
            data,
            temp_data,
        }
    }

    pub fn consume_byte(&mut self) -> Option<u8> {
        if self.index >= self.data.len() {
            return None;
        }
        let byte = self.data[self.index];
        self.index += 1;
        Some(byte)
    }

    pub fn len(&self) -> usize {
        self.data.len() - (self.index as usize)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_index(&self) -> usize {
        self.index
    }

    pub fn get_range(&self, from_index: usize, to_index: usize) -> &[u8] {
        assert!(from_index <= to_index && to_index <= self.data.len());
        &self.data[from_index..to_index]
    }
}

impl<'a, W: Writable> Buffer<'a, W> {
    pub fn write_bytes_to_temp_data(&self, bytes: &[u8]) -> DecodeResult<()> {
        let is_ok = unsafe { self.temp_data.as_mut().unwrap().write(bytes) };
        if is_ok {
            Ok(())
        } else {
            Err(DecodeError::Overflow)
        }
    }
}
