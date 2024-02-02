use core::str::from_utf8;
use ledger_device_sdk::Pic;
use ledger_secure_sdk_sys::nvm_write;
use utils::types::U256;

use crate::error_code::ErrorCode;

#[repr(align(64))]
pub struct NVM<const N: usize>(pub [u8; N]);

impl<const N: usize> NVM<N> {
    pub const fn zeroed() -> Self {
        Self([0; N])
    }

    pub fn write(&mut self, from: usize, slice: &[u8]) -> bool {
        let len = slice.len();
        if from + len > N {
            return false;
        }

        unsafe {
            let dst = self.0[from..].as_mut_ptr() as *mut _;
            let src = slice.as_ptr() as *mut u8 as *mut _;
            nvm_write(dst, src, len as u32); // TODO: handle error properly

            debug_assert_eq!(&self.0[from..], &slice[..]);
        };
        return true;
    }
}

pub struct NvmBuffer<'a, const N: usize> {
    data: &'a mut Pic<NVM<N>>,
    pub index: usize,
}

impl<'a, const N: usize> NvmBuffer<'a, N> {
    pub fn new(data: &'a mut Pic<NVM<N>>) -> Self {
        Self { data, index: 0 }
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<(), ErrorCode> {
        let data = self.data.get_mut();
        if data.write(self.index, bytes) {
            self.index += bytes.len();
            Ok(())
        } else {
            Err(ErrorCode::Overflow)
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.index = 0;
    }
}
