use nanos_sdk::bindings::*;
use core::ptr::null;

#[cfg(feature = "debug")]
pub mod print {

    use nanos_sdk::debug_print;

    pub fn println(s: &str) {
        debug_print(s);
        debug_print("\n");
    }

    pub fn println_slice<const N: usize>(tab: &[u8]) {
        let hex: [u8; N] = utils::to_hex(tab).unwrap();
        let m = core::str::from_utf8(&hex).unwrap();
        println(m);
    }

    pub fn println_array<const N: usize, const M: usize>(tab: &[u8; N]) {
        let hex: [u8; M] = utils::to_hex_fixed::<N, M>(tab).unwrap();
        let m = core::str::from_utf8(&hex).unwrap();
        println(m);
    }
}

#[cfg(feature = "device")]
pub mod print {
    pub fn println(_s: &str) {}
    pub fn println_slice<const N: usize>(_tab: &[u8]) {}
    pub fn println_array<const N: usize, const M: usize>(_tab: &[u8; N]) {}
}

pub fn blake2b(data: &[u8]) -> [u8; 32] {
    print::println("===== a");
    let mut hash = cx_blake2b_t::default();
    let mut result = [0 as u8; 32];
    unsafe {
        print::println("===== b");
        let error = cx_blake2b_init_no_throw(&mut hash, 32 * 8);
        print::println("===== c");
        print::println_array::<4, 8>(&error.to_be_bytes());
        assert!(error == CX_OK);
        let error0 = cx_hash_no_throw(&mut hash.header, 0, data.as_ptr(), data.len() as u32, null::<u8>() as *mut u8, 0);
        assert_eq!(error0, CX_OK);
        print::println("===== d");
        let error1 = cx_hash_no_throw(&mut hash.header, CX_LAST, null(), 0, result.as_mut_ptr(), 32);
        assert_eq!(error1, CX_OK);
        print::println("===== e");
        print::println_slice::<64>(&result)
    }
    return result;
}
