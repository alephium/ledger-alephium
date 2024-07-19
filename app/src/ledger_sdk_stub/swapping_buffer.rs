// This code is inspired by code from zondax: https://github.com/Zondax/ledger-rust/blob/main/bolos/src/swapping_buffer.rs
use utils::buffer::Writable;

use crate::{
    error_code::ErrorCode,
    ledger_sdk_stub::nvm::{NVMData, NVM},
};

pub const RAM_SIZE: usize = 512;

#[derive(Clone, Copy)]
enum BufferState {
    WritingToRam(usize),
    WritingToFlash(usize),
}

impl Default for BufferState {
    fn default() -> Self {
        BufferState::WritingToRam(0)
    }
}

pub struct SwappingBuffer<'a, const RAM: usize, const FLASH: usize> {
    ram: [u8; RAM],
    flash: &'a mut NVMData<NVM<FLASH>>,
    state: BufferState,
}

impl<'a, const RAM: usize, const FLASH: usize> SwappingBuffer<'a, RAM, FLASH> {
    pub fn new(flash: &'a mut NVMData<NVM<FLASH>>) -> Self {
        Self {
            ram: [0u8; RAM],
            flash,
            state: BufferState::default(),
        }
    }

    pub fn read(&self, from_index: usize, to_index: usize) -> &[u8] {
        match self.state {
            BufferState::WritingToRam(_) => {
                assert!(from_index < to_index && to_index <= RAM);
                &self.ram[from_index..to_index]
            }
            BufferState::WritingToFlash(_) => {
                assert!(from_index < to_index && to_index <= FLASH);
                &self.flash.get_ref().0[from_index..to_index]
            }
        }
    }

    pub fn read_all(&self) -> &[u8] {
        match self.state {
            BufferState::WritingToRam(index) => &self.ram[..index],
            BufferState::WritingToFlash(index) => &self.flash.get_ref().0[..index],
        }
    }

    pub fn get_index(&self) -> usize {
        match self.state {
            BufferState::WritingToRam(index) => index,
            BufferState::WritingToFlash(index) => index,
        }
    }

    #[inline]
    fn write_to_ram(&mut self, data: &[u8], from: usize, to: usize) {
        self.ram[from..to].copy_from_slice(data);
        self.state = BufferState::WritingToRam(to);
    }

    #[inline]
    fn write_to_nvm(&mut self, data: &[u8], from: usize) -> Result<(), ErrorCode> {
        self.flash.write_from(from, data)?;
        self.state = BufferState::WritingToFlash(from + data.len());
        Ok(())
    }

    #[inline]
    fn switch_to_nvm(&mut self, ram_length: usize, data: &[u8]) -> Result<(), ErrorCode> {
        self.flash.write_from(0, &self.ram[..ram_length])?;
        self.flash.write_from(ram_length, data)?;
        self.state = BufferState::WritingToFlash(ram_length + data.len());
        Ok(())
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize, ErrorCode> {
        match self.state {
            BufferState::WritingToRam(index) => {
                let to = index + data.len();
                if to <= RAM {
                    self.write_to_ram(data, index, to);
                    return Ok(to);
                }

                self.switch_to_nvm(index, data)?;
                Ok(to)
            }
            BufferState::WritingToFlash(index) => {
                self.write_to_nvm(data, index)?;
                Ok(index + data.len())
            }
        }
    }

    pub fn write_from(&mut self, index: usize, data: &[u8]) -> Result<(), ErrorCode> {
        match self.state {
            BufferState::WritingToRam(_) => {
                let to = index + data.len();
                if to <= RAM {
                    self.write_to_ram(data, index, to);
                    return Ok(());
                }

                self.switch_to_nvm(index, data)
            }
            BufferState::WritingToFlash(_) => self.write_to_nvm(data, index),
        }
    }

    pub fn update(&mut self, from_index: usize, data: &[u8]) {
        let size = data.len();
        match self.state {
            BufferState::WritingToRam(_) => {
                assert!(from_index + size <= RAM);
                self.ram[from_index..(from_index + size)].copy_from_slice(data);
            }
            BufferState::WritingToFlash(_) => {
                assert!(from_index + size <= FLASH);
                self.flash.write_from(from_index, data).unwrap();
            }
        }
    }

    pub fn reset(&mut self) {
        self.state = BufferState::default();
    }
}

impl<'a, const RAM: usize, const FLASH: usize> Writable for SwappingBuffer<'a, RAM, FLASH> {
    fn write(&mut self, bytes: &[u8]) -> bool {
        self.write(bytes).is_ok()
    }
}
