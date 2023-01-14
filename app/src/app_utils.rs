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
}

#[cfg(feature = "device")]
pub mod print {
    pub fn println(_s: &str) {}
    pub fn println_slice<const N: usize>(_tab: &[u8]) {}
}
