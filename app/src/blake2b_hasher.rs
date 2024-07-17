use crate::error_code::ErrorCode;
use ledger_secure_sdk_sys::*;

pub const BLAKE2B_HASH_SIZE: usize = 32;
pub struct Blake2bHasher(cx_blake2b_s);

impl Blake2bHasher {
    pub fn new() -> Self {
        let mut v = cx_blake2b_t::default();
        unsafe { cx_blake2b_init_no_throw(&mut v, BLAKE2B_HASH_SIZE * 8) };
        Self(v)
    }

    pub fn hash(input: &[u8]) -> Result<[u8; BLAKE2B_HASH_SIZE], ErrorCode> {
        let mut hasher = Blake2bHasher::new();
        hasher.update(input)?;
        hasher.finalize()
    }

    pub fn reset(&mut self) {
        unsafe { cx_blake2b_init_no_throw(&mut self.0, BLAKE2B_HASH_SIZE * 8) };
    }

    pub fn update(&mut self, input: &[u8]) -> Result<(), ErrorCode> {
        let rc = unsafe {
            cx_hash_update(
                &mut self.0 as *mut cx_blake2b_s as *mut cx_hash_t,
                input.as_ptr(),
                input.len(),
            )
        };
        if rc == CX_OK {
            Ok(())
        } else {
            Err(ErrorCode::TxSigningFailed)
        }
    }

    pub fn finalize(&mut self) -> Result<[u8; BLAKE2B_HASH_SIZE], ErrorCode> {
        let mut result = [0u8; BLAKE2B_HASH_SIZE];
        let rc = unsafe {
            cx_hash_final(
                &mut self.0 as *mut cx_blake2b_s as *mut cx_hash_t,
                result.as_mut_ptr(),
            )
        };
        if rc == CX_OK {
            Ok(result)
        } else {
            Err(ErrorCode::TxSigningFailed)
        }
    }
}
