use ledger_secure_sdk_sys::nvm_write;

use crate::error_code::ErrorCode;

pub const NVM_DATA_SIZE: usize = 2048;

// The following code is from ledger-rust-sdk
// We've made modifications here to add the `get_mut` functions to `NVMData`

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

// NOTE: `NVMData` is from ledger sdk, we need the `get_ref`
// Needed for `NVMData<T>` to function properly
extern "C" {
    // This is a linker script symbol defining the beginning of
    // the .nvm_data section. Declaring it as a static u32
    // (as is usually done) will result in a r9-indirect memory
    // access, as if it were a RAM access.
    // To force the compiler out of this assumption, we define
    // it as a function instead, but it is _not_ a function at all
    fn _nvm_data_start();
}

/// The following is a means to correctly access data stored in NVM
/// through the `#[link_section = ".nvm_data"]` attribute
pub struct NVMData<T> {
    data: T,
}

impl<T> NVMData<T> {
    pub const fn new(data: T) -> NVMData<T> {
        NVMData { data }
    }

    #[cfg(target_os = "nanos")]
    pub fn get_mut(&mut self) -> &mut T {
        ledger_secure_sdk_sys::pic_rs_mut(&mut self.data)
    }

    #[cfg(target_os = "nanos")]
    pub fn get_ref(&self) -> &T {
        ledger_secure_sdk_sys::pic_rs(&self.data)
    }

    /// This will return a mutable access by casting the pointer
    /// to the correct offset in `.nvm_data` manually.
    /// This is necessary when using the `rwpi` relocation model,
    /// because a static mutable will be assumed to be located in
    /// RAM, and be accessed through the static base (r9)
    #[cfg(not(target_os = "nanos"))]
    fn get_addr(&self) -> *mut T {
        use core::arch::asm;
        unsafe {
            // Compute offset in .nvm_data by taking the reference to
            // self.data and subtracting r9
            let addr = &self.data as *const T as u32;
            let static_base: u32;
            asm!( "mov {}, r9", out(reg) static_base);
            let offset = (addr - static_base) as isize;
            let data_addr = (_nvm_data_start as *const u8).offset(offset);
            ledger_secure_sdk_sys::pic(data_addr as *mut core::ffi::c_void) as *mut T
        }
    }

    #[cfg(not(target_os = "nanos"))]
    pub fn get_mut(&mut self) -> &mut T {
        unsafe {
            let pic_addr = self.get_addr();
            &mut *pic_addr.cast()
        }
    }

    #[cfg(not(target_os = "nanos"))]
    pub fn get_ref(&self) -> &T {
        unsafe {
            let pic_addr = self.get_addr();
            &*pic_addr.cast()
        }
    }
}

impl<const N: usize> NVMData<NVM<N>> {
    pub fn write_from(&mut self, from_index: usize, bytes: &[u8]) -> Result<(), ErrorCode> {
        let data = self.get_mut();
        if data.write(from_index, bytes) {
            Ok(())
        } else {
            Err(ErrorCode::Overflow)
        }
    }
}
