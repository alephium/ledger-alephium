use ledger_device_sdk::NVMData;
use ledger_secure_sdk_sys::nvm_write;

use crate::error_code::ErrorCode;

pub mod swapping_buffer;

pub const NVM_DATA_SIZE: usize = 2048;

#[allow(clippy::upper_case_acronyms)]
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
            nvm_write(dst, src, len as u32);

            assert_eq!(&self.0[from..(from + len)], slice);
        };
        true
    }
}

pub fn write_from<const N: usize>(
    nvm_data: &mut NVMData<NVM<N>>,
    from_index: usize,
    bytes: &[u8],
) -> Result<(), ErrorCode> {
    let data = nvm_data.get_mut();
    if data.write(from_index, bytes) {
        Ok(())
    } else {
        Err(ErrorCode::Overflow)
    }
}
