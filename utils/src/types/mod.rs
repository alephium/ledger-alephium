pub mod asset_output;
pub mod avector;
pub mod bool;
pub mod byte;
pub mod byte32;
mod compact_integer;
pub mod hint;
pub mod i256;
pub mod i32;
pub mod instr;
pub mod lockup_script;
pub mod method;
pub mod public_key;
pub mod script;
pub mod timestamp;
pub mod token;
pub mod tx_input;
pub mod u256;
pub mod unlock_script;
pub mod unsigned_tx;
pub mod val;
#[macro_use]
pub mod macros;
pub mod bigint;
pub mod byte_string;
pub mod method_selector;
pub mod u16;

pub use byte32::Byte32;
pub use i256::I256;
pub use u256::U256;

pub use self::bool::Bool;
pub use self::i32::I32;
pub use self::u16::U16;
pub use asset_output::AssetOutput;
pub use avector::AVector;
pub use bigint::BigInt;
pub use byte::Byte;
pub use byte_string::ByteString;
pub use hint::Hint;
pub use instr::Instr;
pub use lockup_script::LockupScript;
pub use method::Method;
pub use public_key::SecP256K1PubKey;
pub use script::Script;
pub use timestamp::TimeStamp;
pub use token::Token;
pub use tx_input::TxInput;
pub use unlock_script::UnlockScript;
pub use unsigned_tx::UnsignedTx;
pub use val::Val;

pub type Hash = Byte32;

fn reset(dest: &mut [u8]) {
    let mut index = 0;
    while index < dest.len() {
        dest[index] = b'0';
        index += 1;
    }
}

#[cfg(test)]
pub mod test_utils {
    extern crate std;

    use core::fmt::Debug;
    use std::vec::Vec;

    use crate::buffer::Buffer;
    use crate::decode::RawDecoder;
    use crate::decode::{new_decoder, Decoder};
    use crate::types::i32::tests::random_usize;
    use crate::TempData;

    pub fn test_decode<T: Default + RawDecoder + PartialEq + Debug>(
        prefix: u8,
        data: Vec<u8>,
        value: Option<&T>,
        expected_temp_data: Option<&Vec<u8>>,
    ) {
        let bytes = [&[prefix][..], &data[..]].concat();

        {
            let mut temp_data = TempData::new();
            let mut buffer = Buffer::new(&bytes, &mut temp_data);
            let mut decoder = new_decoder::<T>();
            let result = decoder.decode(&mut buffer).unwrap();
            match value {
                Some(_) => assert_eq!(result, value),
                None => assert!(result.is_some()),
            }
        }

        let mut length: usize = 0;
        let mut decoder = new_decoder::<T>();

        let mut temp_data = TempData::new();
        while length < bytes.len() {
            let remain = bytes.len() - length;
            let size = random_usize(0, remain);
            let mut buffer = Buffer::new(&bytes[length..(length + size)], &mut temp_data);
            length += size;

            let result = decoder.decode(&mut buffer).unwrap();
            if length == bytes.len() {
                match value {
                    Some(_) => assert_eq!(result, value),
                    None => assert!(result.is_some()),
                }
                assert!(decoder.stage.is_complete());
                match expected_temp_data {
                    Some(data) => assert_eq!(*data, temp_data.get().to_vec()),
                    None => (),
                }
            } else {
                assert_eq!(result, None);
            }
        }
    }
}
