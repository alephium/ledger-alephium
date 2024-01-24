// from https://github.com/LedgerHQ/app-radix-babylon/blob/develop/src/crypto/hash.rs
use core::ffi::c_uint;
use core::intrinsics::write_bytes;
use core::mem::size_of;
use ledger_secure_sdk_sys::{cx_blake2b_t, cx_md_t, CX_BLAKE2B, CX_OK};

use crate::error_code::ErrorCode;

pub const BLAKE2B_HASH_SIZE: usize = 32;

pub struct Blake2bHasher([u8; Self::WORK_AREA_SIZE]);

extern "C" {
    pub fn cx_hash_init_ex(context: *mut u8, hash_type: cx_md_t, output_size: u32) -> u32;
    pub fn cx_hash_update(hash: *mut u8, input: *const u8, in_len: c_uint) -> u32;
    pub fn cx_hash_final(hash: *mut u8, digest: *mut u8) -> u32;
}

impl Blake2bHasher {
    const WORK_AREA_SIZE: usize = size_of::<cx_blake2b_t>();

    pub fn new() -> Self {
        Self([0; Self::WORK_AREA_SIZE])
    }

    pub fn hash(input: &[u8]) -> Result<[u8; BLAKE2B_HASH_SIZE], ErrorCode> {
        let mut hasher = Blake2bHasher::new();
        hasher.init()?;
        hasher.update(input)?;
        hasher.finalize()
    }

    pub fn reset(&mut self) {
        unsafe {
            write_bytes(self, 0, 1);
        }
    }

    pub fn init(&mut self) -> Result<(), ErrorCode> {
        self.reset();
        let rc =
            unsafe { cx_hash_init_ex(self.0.as_mut_ptr(), CX_BLAKE2B, BLAKE2B_HASH_SIZE as u32) };
        if rc == CX_OK {
            Ok(())
        } else {
            Err(ErrorCode::TxSignFail)
        }
    }

    pub fn update(&mut self, input: &[u8]) -> Result<(), ErrorCode> {
        let rc =
            unsafe { cx_hash_update(self.0.as_mut_ptr(), input.as_ptr(), input.len() as c_uint) };
        if rc == CX_OK {
            Ok(())
        } else {
            Err(ErrorCode::TxSignFail)
        }
    }

    pub fn finalize(&mut self) -> Result<[u8; BLAKE2B_HASH_SIZE], ErrorCode> {
        let mut result = [0u8; BLAKE2B_HASH_SIZE];
        let rc = unsafe { cx_hash_final(self.0.as_mut_ptr(), result.as_mut_ptr()) };
        if rc == CX_OK {
            Ok(result)
        } else {
            Err(ErrorCode::TxSignFail)
        }
    }
}
