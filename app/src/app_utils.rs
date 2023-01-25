use nanos_sdk::bindings::*;

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
    let mut hash = cx_hash_header_s::default();
    let mut result = [0 as u8; 32];
    unsafe {
        let error = cx_hash_init_ex(&mut hash, CX_BLAKE2B, 32);
        assert!(error == 0);
        cx_hash_update(&mut hash, data.as_ptr(), data.len() as u32);
        cx_hash_final(&mut hash, result.as_mut_ptr());
    }
    return result;
}

pub fn djb_hash(data: &[u8]) -> i32 {
    let mut hash: i32 = 5381;
    data.into_iter().for_each(|&byte| {
        hash = ((hash << 5) + hash) + (byte as i32);
    });
    return hash;
}

pub fn xor_bytes(data: i32) -> u8 {
    let bytes = data.to_be_bytes();
    return bytes[0] ^ bytes[1] ^ bytes[2] ^ bytes[3];
}
