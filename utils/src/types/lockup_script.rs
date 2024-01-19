use crate::decode::*;
use crate::buffer::Buffer;
use super::Hash;

#[cfg_attr(test, derive(Debug))]
#[derive(PartialEq)]
pub enum LockupScript {
  P2PKH(Hash),
  Unknown
}

impl Default for LockupScript {
  fn default() -> Self { LockupScript::Unknown }
}

impl RawDecoder for LockupScript {
  fn step_size(&self) -> usize { 1 }

  fn decode<'a>(&mut self, buffer: &mut Buffer<'a>, stage: &DecodeStage) -> DecodeResult<DecodeStage> {
    if buffer.is_empty() {
      return Ok(DecodeStage { ..*stage });
    }
    if *self == LockupScript::Unknown {
      let tpe = buffer.next_byte().unwrap();
      if tpe != 0 {
        return Err(DecodeError::NotSupported);
      }
      *self = LockupScript::P2PKH(Hash::default());
    }
    match self {
      Self::P2PKH(hash) => hash.decode(buffer, stage),
      Self::Unknown => Err(DecodeError::InternalError),
    }
  }
}

#[cfg(test)]
mod tests {
  extern crate std;

  use std::vec;
  use crate::buffer::Buffer;
  use crate::decode::{Decoder, DecodeError, new_decoder};
  use crate::types::{Hash, LockupScript};
  use crate::types::i32::tests::random_usize;
  use crate::types::byte32::tests::gen_bytes;

  #[test]
  fn test_decode_p2pkh() {
    for _ in 0..10 {
      let mut bytes = vec![0u8];
      let hash_bytes = gen_bytes(32, 32);
      bytes.extend(&hash_bytes);
      let lockup_script = LockupScript::P2PKH(Hash::from_bytes(hash_bytes.as_slice().try_into().unwrap()));

      {
        let mut buffer = Buffer::new(&bytes).unwrap();
        let mut decoder = new_decoder::<LockupScript>();
        let result = decoder.decode(&mut buffer).unwrap();
        assert_eq!(result, Some(&lockup_script));
      }

      let mut length: usize = 0;
      let mut decoder = new_decoder::<LockupScript>();

      while length < bytes.len() {
        let remain = bytes.len() - length;
        let size = random_usize(0, remain);
        let mut buffer = Buffer::new(&bytes[length..(length+size)]).unwrap();
        length += size;

        let result = decoder.decode(&mut buffer).unwrap();
        if length == bytes.len() {
          assert_eq!(result, Some(&lockup_script));
          assert!(decoder.stage.is_complete())
        } else {
          assert_eq!(result, None);
        }
      }
    }
  }

  #[test]
  fn test_decode_invalid_lockup_script() {
    let invalid_types = [1u8, 2u8, 3u8];
    for tpe in invalid_types {
      let bytes = vec![tpe];
      let mut buffer = Buffer::new(&bytes).unwrap();
      let mut decoder = new_decoder::<LockupScript>();
      let result = decoder.decode(&mut buffer);
      assert!(result.is_err());
      assert_eq!(result.unwrap_err(), DecodeError::NotSupported);
    }
  }
}
