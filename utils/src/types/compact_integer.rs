static ONE_BYTE_PREFIX: u8 = 0x00;
static TWO_BYTE_PREFIX: u8 = 0x40;
static FOUR_BYTE_PREFIX: u8 = 0x80;

pub static MASK_MODE: u32 = 0x3f;
static MASK_REST: u32 = 0xc0;
pub static MASK_MODE_NEG: u32 = 0xffffffc0;

pub fn is_fixed_size(byte: u8) -> bool {
    let value = byte as u32;
    let prefix = (value & MASK_REST) as u8;
    prefix == ONE_BYTE_PREFIX || prefix == TWO_BYTE_PREFIX || prefix == FOUR_BYTE_PREFIX
}

pub fn decode_length(byte: u8) -> usize {
    let value = byte as u32;
    let prefix = (value & MASK_REST) as u8;
    if prefix == ONE_BYTE_PREFIX {
        1
    } else if prefix == TWO_BYTE_PREFIX {
        2
    } else if prefix == FOUR_BYTE_PREFIX {
        4
    } else {
        ((value & MASK_MODE) + 4 + 1) as usize
    }
}
