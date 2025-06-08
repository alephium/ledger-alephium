use crate::error_code::ErrorCode;

#[repr(u8)]
pub enum KeyType {
    Default,
    GLSecp256k1,
}

impl KeyType {
    pub fn from(byte: u8) -> Result<KeyType, ErrorCode> {
        match byte {
            0 => Ok(KeyType::Default),
            1 => Ok(KeyType::GLSecp256k1),
            _ => Err(ErrorCode::InvalidKeyType),
        }
    }
}
