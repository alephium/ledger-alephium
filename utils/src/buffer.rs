use crate::decode::{DecodeError, DecodeResult};

pub trait Writable {
    fn write(&mut self, bytes: &[u8]) -> bool;
}

pub struct Buffer<'a, W> {
    index: u8,
    data: &'a [u8],
    temp_data: *mut W,
}

impl<'a, W> Buffer<'a, W> {
    pub fn new(data: &'a [u8], temp_data: *mut W) -> Option<Buffer<'a, W>> {
        if data.len() > (u8::MAX as usize) {
            return None;
        }
        Some(Buffer {
            index: 0,
            data,
            temp_data,
        })
    }

    pub fn next_byte(&mut self) -> Option<u8> {
        let idx = self.index as usize;
        if idx >= self.data.len() {
            return None;
        }
        let byte = self.data[idx];
        self.index += 1;
        Some(byte)
    }

    pub fn len(&self) -> usize {
        self.data.len() - (self.index as usize)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_index(&self) -> u8 {
        self.index
    }

    pub fn get_range(&self, from_index: u8, to_index: u8) -> &[u8] {
        &self.data[(from_index as usize)..(to_index as usize)]
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
