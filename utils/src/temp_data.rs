pub struct TempData {
    pub data: [u8; TempData::MAX_SIZE],
    pub size: usize,
}

impl TempData {
    pub const MAX_SIZE: usize = 160;

    pub fn new() -> Self {
        Self {
            data: [0; TempData::MAX_SIZE],
            size: 0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.data = [0; TempData::MAX_SIZE];
        self.size = 0;
    }

    #[inline]
    pub fn is_overflow(&self) -> bool {
        self.size == Self::MAX_SIZE
    }

    #[inline]
    pub fn write_byte(&mut self, byte: u8) {
        if self.size == TempData::MAX_SIZE {
            return;
        }
        self.data[self.size] = byte;
        self.size += 1;
    }

    pub fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.write_byte(*byte)
        }
    }

    pub fn get(&self) -> &[u8] {
        &self.data[..self.size]
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use crate::types::byte32::tests::gen_bytes;

    use super::TempData;
    use std::vec::Vec;

    fn gen_fixed_bytes(size: usize) -> Vec<u8> {
        gen_bytes(size, size)
    }

    #[test]
    fn test_write() {
        let mut temp_data = TempData::new();
        assert!(!temp_data.is_overflow());
        assert_eq!(temp_data.size, 0);
        assert_eq!(temp_data.data, [0u8; TempData::MAX_SIZE]);

        let bytes0 = gen_fixed_bytes(10);
        temp_data.write(&bytes0);
        assert_eq!(temp_data.size, 10);
        assert_eq!(&temp_data.data[..10], &bytes0);

        let bytes1 = gen_fixed_bytes(15);
        temp_data.write(&bytes1);
        assert_eq!(temp_data.size, 25);
        assert_eq!(&temp_data.data[..10], &bytes0);
        assert_eq!(&temp_data.data[10..25], &bytes1);

        let bytes2 = gen_fixed_bytes(TempData::MAX_SIZE - 25 - 1);
        temp_data.write(&bytes2);
        assert_eq!(temp_data.size, TempData::MAX_SIZE - 1);
        assert!(!temp_data.is_overflow());
        assert_eq!(&temp_data.data[..10], &bytes0);
        assert_eq!(&temp_data.data[10..25], &bytes1);
        assert_eq!(&temp_data.data[25..(TempData::MAX_SIZE - 1)], &bytes2);

        let bytes3 = gen_fixed_bytes(1);
        temp_data.write(&bytes3);
        assert_eq!(temp_data.size, TempData::MAX_SIZE);
        assert!(temp_data.is_overflow());

        temp_data.reset();
        assert_eq!(temp_data.size, 0);
        assert!(!temp_data.is_overflow());
        assert_eq!(temp_data.data, [0u8; TempData::MAX_SIZE]);
    }
}
