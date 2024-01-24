pub mod u256;
pub mod byte32;
pub mod i32;
pub mod timestamp;
pub mod token;
pub mod avector;
pub mod byte;
pub mod asset_output;
pub mod lockup_script;
pub mod public_key;
pub mod hint;
pub mod unlock_script;
pub mod tx_input;
pub mod unsigned_tx;
mod compact_integer;

pub use u256::U256;
pub use byte32::Byte32;

pub use self::i32::I32;
pub use timestamp::TimeStamp;
pub use token::Token;
pub use avector::AVector;
pub use byte::Byte;
pub use asset_output::AssetOutput;
pub use lockup_script::LockupScript;
pub use public_key::PublicKey;
pub use hint::Hint;
pub use unlock_script::UnlockScript;
pub use tx_input::TxInput;
pub use unsigned_tx::UnsignedTx;

pub type Hash = Byte32;
pub type ByteString = AVector<Byte>;

fn reset<const NUM: usize>(dest: &mut [u8; NUM]) {
  let mut index = 0;
  while index < dest.len() {
    dest[index] = b'0';
    index += 1;
  }
}

pub fn extend_slice<const NUM: usize>(dest: &mut [u8; NUM], from_index: usize, source: &[u8]) -> usize {
  let mut index = 0;
  while index < source.len() {
    dest[index + from_index] = source[index];
    index += 1;
  }
  from_index + index
}