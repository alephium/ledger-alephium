use crate::decode::*;
use crate::buffer::Buffer;
use super::*;

#[cfg_attr(test, derive(Debug))]
#[derive(Default)]
pub struct UnsignedTx {
  pub version: Byte,
  pub network_id: Byte,
  // script_opt: None,
  pub gas_amount: I32,
  pub gas_price: U256,
  pub inputs: AVector<TxInput>,
  pub fixed_outputs: AVector<AssetOutput>,
}

impl UnsignedTx {
  fn decode_script_opt(&mut self, buffer: &mut Buffer, stage: &DecodeStage) -> DecodeResult<DecodeStage> {
    if buffer.is_empty() {
      return Ok(DecodeStage { ..*stage });
    }
    let byte = buffer.next_byte().unwrap();
    if byte != 0 {
      Err(DecodeError::NotSupported)
    } else {
      Ok(DecodeStage::COMPLETE)
    }
  }
}

impl RawDecoder for UnsignedTx {
  fn step_size(&self) -> usize {
    5 + self.inputs.step_size() + self.fixed_outputs.step_size()
  }

  fn decode<'a>(&mut self, buffer: &mut Buffer<'a>, stage: &DecodeStage) -> DecodeResult<DecodeStage> {
    match stage.step {
      0 => self.version.decode(buffer, stage),
      1 => self.network_id.decode(buffer, stage),
      2 => self.decode_script_opt(buffer, stage),
      3 => self.gas_amount.decode(buffer, stage),
      4 => self.gas_price.decode(buffer, stage),
      step => {
        if step > 4 && step <= (4 + self.inputs.step_size()) {
          self.inputs.decode(buffer, stage)
        } else if step <= self.step_size() {
          self.fixed_outputs.decode(buffer, stage)
        } else {
          Err(DecodeError::InternalError)
        }
      },
    }
  }
}

#[cfg(test)]
mod tests {
  extern crate std;
  extern crate alloc;

  use alloc::string::String;
  use blake2::{Blake2b, Digest};
  use digest::consts::U32;
  use core::cmp::min;
  use std::vec::Vec;
  use std::vec;
  use num_bigint::BigUint;
  use crate::buffer::Buffer;
  use crate::decode::{Decoder, DecodeError, new_decoder};
  use crate::types::{I32, U256, Hint, UnlockScript, PublicKey, Hash, LockupScript};
  use crate::types::i32::tests::random_usize;
  use crate::types::u256::tests::hex_to_bytes;
  use super::{UnsignedTx, PartialDecoder};

  type Blake2b256 = Blake2b<U32>;

  pub struct TxDecodeContext {
    decoder: PartialDecoder<UnsignedTx>,
    hasher: Blake2b256,
  }

  impl TxDecodeContext {
    pub fn decode<'a>(&mut self, buffer: &mut Buffer<'a>) -> Result<Option<&UnsignedTx>, DecodeError>  {
      let from_index = buffer.get_index();
      let result = self.decoder.decode(buffer);
      let to_index = buffer.get_index();
      self.hasher.update(buffer.get_range(from_index, to_index));
      result
    }

    pub fn get_tx_id(self) -> Option<Vec<u8>> {
      if self.decoder.stage.is_complete() {
        Some(self.hasher.finalize().to_vec())
      } else {
        None
      }
    }

    pub fn is_complete(&self) -> bool {
      self.decoder.stage.is_complete()
    }
  }

  impl Default for TxDecodeContext {
    fn default() -> Self {
      TxDecodeContext {
        decoder: new_decoder(),
        hasher: Blake2b256::new(),
      }
    }
  }

  fn u256_to_string(u256: &U256) -> String {
    let mut bytes: Vec<u8> = vec![];
    for n in u256.inner {
      bytes.extend(n.to_be_bytes());
    }
    let big_int = BigUint::from_bytes_be(&bytes);
    return big_int.to_str_radix(10);
  }

  #[test]
  fn test_decode_transfer_alph_tx() {
    let tx_id_hex = "c53f150bceb13c6ca1c13fee897e688c0ef86c73ad8113edf444b7b15ecf438b";
    let bytes = hex_to_bytes("0000008000de1cc1174876e80006cb6501716c7f09df51c6e9d2412210f756dd13b12914ace98be11a506468bbc09b4457f30002622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235ecb650171c5b1c7ec8f38a446b5824ab3b4785eb59813be6309caccf09e81badba48875550002622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235ecb650171c8fc4448bd13db645484b628da13e8e95d0c1c7f63d93e2d2098dac7c902dec30002622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235ecb6501715faa376ca823d5a3bf265ff932e3ddc695b87d7d577e6c77277a96756d42cd430002622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235ecb6501716fc17b71c1a8be6f822b74d991675535cb55af5835d7f2ed146f769323c3e9450002622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235ecb650171950bf46c8d7fe6ca54a2cffdbc29f60c9b666fb42cb1c09a17d2ff555e3e893e0002622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235e07c4145b402ea4c0cb000038f63ae3338e738b288103aa3d4cab822a8bfaf19ace50798bd4c8439f06c55700000000000000000000c40eb17f1ebec364c000f933eafd1dd5d5ac00d6eac5dd0f54e527e72aa8d82f81701ae6b8e481d9708500000000000000000000c40ed336ec389dffc0002f53372b89cbe04a208643ccf098561ea545fdb121359df48378e828dbb3ef1100000000000000000000c48b127aec9cc8068000102bdf758a5fb7c1f049e75c7d297f1aa7d84d74eeaf9cee2b388d1fc94ec48000000000000000000000c40de259e640f7c040007720aecb72dfa949eefe173bdff8223346384b564389533bd267ecdfe8dcdadc00000000000000000000c40e4568375f83f5c000df1562ff1670a6d955d1f7c27d6319289b1fc358bf357adf97d5f097a6895f0a00000000000000000000c44ec157b933227c80009b85f066b1b2821339bf73e9e00bbe660b0cfb97158ceedff3260e1e4368961d00000000000000000000").unwrap();

    let check = |expected: &UnsignedTx| {
      assert_eq!(expected.version.0, 0);
      assert_eq!(expected.network_id.0, 0);
      assert_eq!(expected.gas_amount, I32::from(56860));
      assert_eq!(expected.gas_price, U256::from_u64(100000000000));
      assert_eq!(expected.inputs.size(), 6);
      assert_eq!(expected.fixed_outputs.size(), 7);
      assert!(expected.inputs.is_complete());
      assert!(expected.fixed_outputs.is_complete());

      let last_input = expected.inputs.get_current_item().unwrap();
      let input_hint_bytes = i32::to_be_bytes(-882572943);
      let output_ref_key_bytes = hex_to_bytes("950bf46c8d7fe6ca54a2cffdbc29f60c9b666fb42cb1c09a17d2ff555e3e893e").unwrap();
      let public_key_bytes = hex_to_bytes("02622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235e").unwrap();
      assert_eq!(last_input.hint, Hint::from_bytes(input_hint_bytes));
      assert_eq!(last_input.key, Hash::from_bytes(output_ref_key_bytes.as_slice().try_into().unwrap()));
      assert_eq!(last_input.unlock_script, UnlockScript::P2PKH(PublicKey::from_bytes(public_key_bytes.as_slice().try_into().unwrap())));

      let last_output = expected.fixed_outputs.get_current_item().unwrap();
      let atto_alph_amount = String::from("5674913458402000000");
      let public_key_hash_bytes = hex_to_bytes("9b85f066b1b2821339bf73e9e00bbe660b0cfb97158ceedff3260e1e4368961d").unwrap();
      assert_eq!(u256_to_string(&last_output.amount), atto_alph_amount);
      assert_eq!(last_output.lockup_script, LockupScript::P2PKH(Hash::from_bytes(public_key_hash_bytes.as_slice().try_into().unwrap())));
      assert!(last_output.tokens.is_empty());
      assert!(last_output.additional_data.is_empty());
    };

    let mut length: usize = 0;
    let mut decoder = TxDecodeContext::default();

    while length < bytes.len() {
      let remain = bytes.len() - length;
      let size = min(random_usize(0, u8::MAX as usize), remain);
      let mut buffer = Buffer::new(&bytes[length..(length+size)]).unwrap();
      length += size;

      let result = decoder.decode(&mut buffer).unwrap();
      if length == bytes.len() {
        assert!(buffer.is_empty());
        assert!(result.is_some());
        check(result.unwrap());
        assert!(decoder.is_complete());
      } else {
        let is_none = result.is_none();
        assert!(is_none);
        assert!(!decoder.is_complete());
      }
    }

    let tx_id = decoder.get_tx_id().unwrap();
    assert_eq!(hex_to_bytes(tx_id_hex).unwrap(), tx_id);
  }

  #[test]
  fn test_decode_transfer_token_tx() {
    let tx_id_hex = "668827ae5719d8acb7efa4e8684cd3968738736833369ad56482b7ccb6bad5c7";
    let bytes = hex_to_bytes("000000800079ccc1174876e80003f6179435b26eb070309593a0aa5eef3f1ae3f7337a0dba1e7d94f3d8c4adc2743636057c0002e835a6e954a0a0b0e540f4451186e5a1f99baf93a111d304866945a768c39d5cf61794350817b6c1ea8fae4a48fb6868d8f47147ef8bd62a92589a876419352dfc5103610002e835a6e954a0a0b0e540f4451186e5a1f99baf93a111d304866945a768c39d5cf61794353cfed394414a0238ab8be798b88140c4f9255f094f30614f184afa0ba5984ba00002e835a6e954a0a0b0e540f4451186e5a1f99baf93a111d304866945a768c39d5c04c3038d7ea4c6800000bee85f379545a2ed9f6cceb331288842f378cf0f04012ad4ac8824aae7d6f80a0000000000000000011a281053ba8601a658368594da034c2e99a0fb951b86498d05e76aedfe666800c3038d7ea4c6800000c40c79e3bca513800000bee85f379545a2ed9f6cceb331288842f378cf0f04012ad4ac8824aae7d6f80a00000000000000000000c3038d7ea4c68000004e796b6f3b889eb8959c285ea4ef8dea6d7aad4c444e2f83f3403fdfde5d2eb60000000000000000011a281053ba8601a658368594da034c2e99a0fb951b86498d05e76aedfe666800c302dd4700d857d600c438a38658095af000004e796b6f3b889eb8959c285ea4ef8dea6d7aad4c444e2f83f3403fdfde5d2eb600000000000000000000").unwrap();
    let check = |expected: &UnsignedTx| {
      assert_eq!(expected.version.0, 0);
      assert_eq!(expected.network_id.0, 0);
      assert_eq!(expected.gas_amount, I32::from(31180));
      assert_eq!(expected.gas_price, U256::from_u64(100000000000));
      assert_eq!(expected.inputs.size(), 3);
      assert_eq!(expected.fixed_outputs.size(), 4);
      assert!(expected.inputs.is_complete());
      assert!(expected.fixed_outputs.is_complete());

      let last_input = expected.inputs.get_current_item().unwrap();
      let input_hint_bytes = i32::to_be_bytes(-166226891);
      let output_ref_key_bytes = hex_to_bytes("3cfed394414a0238ab8be798b88140c4f9255f094f30614f184afa0ba5984ba0").unwrap();
      let public_key_bytes = hex_to_bytes("02e835a6e954a0a0b0e540f4451186e5a1f99baf93a111d304866945a768c39d5c").unwrap();
      assert_eq!(last_input.hint, Hint::from_bytes(input_hint_bytes));
      assert_eq!(last_input.key, Hash::from_bytes(output_ref_key_bytes.as_slice().try_into().unwrap()));
      assert_eq!(last_input.unlock_script, UnlockScript::P2PKH(PublicKey::from_bytes(public_key_bytes.as_slice().try_into().unwrap())));

      let last_output = expected.fixed_outputs.get_current_item().unwrap();
      let atto_alph_amount = String::from("4081253400000000000");
      let public_key_hash_bytes = hex_to_bytes("4e796b6f3b889eb8959c285ea4ef8dea6d7aad4c444e2f83f3403fdfde5d2eb6").unwrap();
      assert_eq!(u256_to_string(&last_output.amount), atto_alph_amount);
      assert_eq!(last_output.lockup_script, LockupScript::P2PKH(Hash::from_bytes(public_key_hash_bytes.as_slice().try_into().unwrap())));
      assert!(last_output.tokens.is_empty());
      assert!(last_output.additional_data.is_empty());
    };

    let mut length: usize = 0;
    let mut decoder = TxDecodeContext::default();

    while length < bytes.len() {
      let remain = bytes.len() - length;
      let size = min(random_usize(0, u8::MAX as usize), remain);
      let mut buffer = Buffer::new(&bytes[length..(length+size)]).unwrap();
      length += size;

      let result = decoder.decode(&mut buffer).unwrap();
      if length == bytes.len() {
        assert!(buffer.is_empty());
        assert!(result.is_some());
        check(result.unwrap());
        assert!(decoder.is_complete());
      } else {
        assert!(result.is_none());
        assert!(!decoder.is_complete());
      }
    }

    let tx_id = decoder.get_tx_id().unwrap();
    assert_eq!(hex_to_bytes(tx_id_hex).unwrap(), tx_id);
  }

  #[test]
  fn test_script_tx() {
    // b4d93868e9b20c2757067334799ea815614fcec306eb254832dbbbd58eb8d42a
    let bytes = hex_to_bytes("0000010101030001000b1440205bf2f559ae714dab83ff36bed4d9e634dfda3ca9ed755d60f00be89e2a20bd001700b4160013c5056bc75e2d63100000a313c5056bc75e2d631000000d0c1440205bf2f559ae714dab83ff36bed4d9e634dfda3ca9ed755d60f00be89e2a20bd00010e8000bffcc1174876e80002e412bbf9030c20b11b0d1755c76eca9aee0144286933d46bfadbdd0b59976ae73e67523000037fda053ebb06b77a9b03ba029f826ec3e1337e47462743bc0b5035ec0d033615e412bbf93f98f4e88567ca1b978d5a59b126fa8afd7432231c8217e2684e99d3d686826e00037fda053ebb06b77a9b03ba029f826ec3e1337e47462743bc0b5035ec0d03361502c3038d7ea4c68000005bb4d7a6644d4981818916b1d480335290ec9c38beacb827fe92dde7cab5698d0000000000000000015bf2f559ae714dab83ff36bed4d9e634dfda3ca9ed755d60f00be89e2a20bd00c50d49f0894c3e0c685800c530759dc0cd56ff0000005bb4d7a6644d4981818916b1d480335290ec9c38beacb827fe92dde7cab5698d00000000000000000000").unwrap();
    let mut buffer = Buffer::new(&bytes[..(u8::MAX as usize)]).unwrap();
    let mut decoder = new_decoder::<UnsignedTx>();
    let result = decoder.decode(&mut buffer);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), DecodeError::NotSupported);
  }
}