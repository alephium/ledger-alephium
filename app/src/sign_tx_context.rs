use ledger_device_sdk::io::ApduHeader;
use ledger_device_sdk::ui::gadgets::{Field, MultiFieldReview};
use ledger_device_sdk::ui::bitmaps::{EYE, CHECKMARK, CROSS};
use utils::base58::base58_encode;
use core::str::from_utf8;
use utils::types::{extend_slice, AssetOutput, Hash, LockupScript, TxInput, UnlockScript, I32, U256};
use utils::{decode::PartialDecoder, types::UnsignedTx, deserialize_path, buffer::Buffer};

use crate::{blake2b_hasher::{Blake2bHasher, BLAKE2B_HASH_SIZE}, error_code::ErrorCode};

#[derive(PartialEq)]
enum DecodeStep {
  Init,
  DerivePath,
  TxVersion,
  TxNetworkId,
  TxScriptOpt,
  TxGasAmount,
  TxGasPrice,
  TxInputs,
  TxOutputs,
  Complete,
}

pub struct SignTxContext {
  pub path: [u32; 5],
  unsigned_tx: PartialDecoder<UnsignedTx>,
  current_step: DecodeStep,
  hasher: Blake2bHasher,
}


impl SignTxContext {
  pub fn new() -> Result<Self, ErrorCode> {
    let mut hasher = Blake2bHasher::new();
    hasher.init()?;
    Ok(SignTxContext {
      path: [0; 5],
      unsigned_tx: PartialDecoder::default(),
      current_step: DecodeStep::Init,
      hasher,
    })
  }

  fn next_step(&mut self) {
    let next = match self.current_step {
      DecodeStep::Init => DecodeStep::DerivePath,
      DecodeStep::DerivePath => DecodeStep::TxVersion,
      DecodeStep::TxVersion => DecodeStep::TxNetworkId,
      DecodeStep::TxNetworkId => DecodeStep::TxScriptOpt,
      DecodeStep::TxScriptOpt => DecodeStep::TxGasAmount,
      DecodeStep::TxGasAmount => DecodeStep::TxGasPrice,
      DecodeStep::TxGasPrice => DecodeStep::TxInputs,
      DecodeStep::TxInputs => {
        if self.unsigned_tx.inner.inputs.is_complete() {
          DecodeStep::TxOutputs
        } else {
          DecodeStep::TxInputs
        }
      },
      DecodeStep::TxOutputs => {
        if self.unsigned_tx.inner.fixed_outputs.is_complete() {
          DecodeStep::Complete
        } else {
          DecodeStep::TxOutputs
        }
      },
      DecodeStep::Complete => DecodeStep::Complete
    };
    self.current_step = next;
  }

  pub fn is_complete(&self) -> bool {
    self.current_step == DecodeStep::Complete
  }

  pub fn get_tx_id(&mut self) -> Result<[u8; BLAKE2B_HASH_SIZE], ErrorCode> {
    self.hasher.finalize()
  }

  fn display(&mut self) -> Result<(), ErrorCode> {
    match self.current_step {
      DecodeStep::TxNetworkId => display_network(self.unsigned_tx.inner.network_id.0),
      DecodeStep::TxGasAmount => display_gas_amount(&self.unsigned_tx.inner.gas_amount),
      DecodeStep::TxGasPrice => display_gas_price(&self.unsigned_tx.inner.gas_price),
      DecodeStep::TxInputs => {
        let current_input = self.unsigned_tx.inner.inputs.get_current_item();
        if current_input.is_some() {
          let current_index = self.unsigned_tx.inner.inputs.current_index;
          display_tx_input(current_input.unwrap(), current_index as usize)
        } else {
          Ok(())
        }
      },
      DecodeStep::TxOutputs => {
        let current_output = self.unsigned_tx.inner.fixed_outputs.get_current_item();
        if current_output.is_some() {
          let current_index = self.unsigned_tx.inner.fixed_outputs.current_index;
          display_tx_output(current_output.unwrap(), current_index as usize)
        } else {
          Ok(())
        }
      },
      _ => Ok(())
    }
  }

  fn _decode_tx(&mut self, buffer: &mut Buffer) -> Result<(), ErrorCode> {
    match self.unsigned_tx.try_decode_one_step(buffer) {
      Ok(true) => {
        self.display()?;
        self.next_step();
        if self.unsigned_tx.stage.is_complete() {
          self.current_step = DecodeStep::Complete;
          Ok(())
        } else {
          self._decode_tx(buffer)
        }
      }
      Ok(false) => Ok(()),
      Err(_) => Err(ErrorCode::TxDecodeFail),
    }
  }

  fn decode_tx(&mut self, data: &[u8]) -> Result<(), ErrorCode> {
    if data.len() > (u8::MAX as usize) {
      return Err(ErrorCode::BadLen)
    }
    let mut buffer = Buffer::new(data).unwrap();
    let from_index = buffer.get_index();
    let result = self._decode_tx(&mut buffer);
    let to_index = buffer.get_index();
    self.hasher.update(buffer.get_range(from_index, to_index))?;
    result
  }

  pub fn handle_data(&mut self, apdu_header: &ApduHeader, data: &[u8]) -> Result<(), ErrorCode> {
    match self.current_step {
      DecodeStep::Complete => Err(ErrorCode::InternalError),
      DecodeStep::Init =>
        if apdu_header.p1 == 0 && data.len() >= 20 {
          if !deserialize_path(&data[0..20], &mut self.path) {
            return Err(ErrorCode::DerivePathDecodeFail);
          }
          self.current_step = DecodeStep::TxVersion;
          self.decode_tx(&data[20..])
        } else {
          Err(ErrorCode::TxDecodeFail)
        },
      _ => 
        if apdu_header.p1 == 1 {
          self.decode_tx(data)
        } else {
          Err(ErrorCode::TxDecodeFail)
        }
    }
  }
}

fn display_network(id: u8) -> Result<(), ErrorCode> {
  let network_type = match id {
    0 => "mainnet",
    1 => "testnet",
    _ => "devnet"
  };

  let fields = [Field { name: "Network", value: network_type }];
  display(&fields, "Review Network")
}

#[inline]
fn bytes_to_string(bytes: &[u8]) -> Result<&str, ErrorCode> {
  from_utf8(bytes).map_err(|_| ErrorCode::InvalidParameter)
}

fn num_with_prefix<'a, const NUM: usize>(prefix: &[u8], num: &I32, output: &'a mut [u8; NUM]) -> Result<&'a str, ErrorCode> {
  if NUM < 11 + prefix.len() { return Err(ErrorCode::Overflow); }
  let mut num_output: [u8; 11] = output[prefix.len()..].try_into().map_err(|_| ErrorCode::Overflow)?;
  let num_str_bytes = num.to_str(&mut num_output);
  if num_str_bytes.is_none() { return Err(ErrorCode::Overflow); }
  let mut size = extend_slice(output, 0, prefix);
  size = extend_slice(output, size, num_str_bytes.unwrap());
  bytes_to_string(&output[..size])
}

fn to_alph_str<'a, const NUM: usize>(amount: &U256, output: &'a mut [u8; NUM]) -> Result<&'a str, ErrorCode> {
  let post_fix = b" ALPH";
  if NUM < 17 + post_fix.len() { return Err(ErrorCode::Overflow) }
  let mut num_output: [u8; 17] = output[0..17].try_into().map_err(|_| ErrorCode::Overflow)?;
  let str_bytes = amount.to_alph(&mut num_output);
  if str_bytes.is_none() { return Err(ErrorCode::Overflow); }
  let mut size = extend_slice(output, 0, str_bytes.unwrap());
  size = extend_slice(output, size, post_fix);
  bytes_to_string(&output[..size])
}

fn to_address<'a, const NUM: usize>(prefix: u8, hash: &Hash, output: &'a mut [u8; NUM]) -> Result<&'a str, ErrorCode> {
  let mut encoded = [0u8; 33];
  encoded[0] = prefix;
  extend_slice(&mut encoded, 1, &hash.0);
  let str_bytes = base58_encode(&encoded, output);
  if str_bytes.is_none() { return Err(ErrorCode::Overflow); }
  bytes_to_string(str_bytes.unwrap())
}

fn display_gas_amount(gas_amount: &I32) -> Result<(), ErrorCode> {
  let mut output = [0; 11];
  let value = num_with_prefix(b"", gas_amount, &mut output)?;
  let fields = [Field { name: "GasAmount", value }];
  display(&fields, "Review Gas Amount")
}

fn display_gas_price(gas_price: &U256) -> Result<(), ErrorCode> {
  let mut output = [0; 22];
  let value = to_alph_str(gas_price, &mut output)?;
  let fields = [Field { name: "GasPrice", value }];
  display(&fields, "Review Gas Price")
}

fn display_tx_input(tx_input: &TxInput, current_index: usize) -> Result<(), ErrorCode> {
  match &tx_input.unlock_script {
    UnlockScript::P2PKH(public_key) => {
      let public_key_hash = Blake2bHasher::hash(&public_key.0)?;
      let mut bytes = [0u8; 50];
      let value = to_address(0u8, &Hash::from_bytes(public_key_hash), &mut bytes)?;
      let fields = [Field { name: "Address", value }];
      let mut review_message_bytes = [0u8; 25]; // b"Review Input #".len() + 11
      let review_message = num_with_prefix(b"Review Input #", &I32::unsafe_from(current_index), &mut review_message_bytes)?;
      return display(&fields, review_message);
    },
    _ => return Err(ErrorCode::NotSupported),
  };
}

fn display_tx_output(output: &AssetOutput, current_index: usize) -> Result<(), ErrorCode> {
  let mut amount_output = [0u8; 22];
  let amount_str = to_alph_str(&output.amount, &mut amount_output)?;
  let amount_field = Field { name: "Amount", value: amount_str };
  match &output.lockup_script {
    LockupScript::P2PKH(public_key_hash) => {
      let mut bytes = [0u8; 50];
      let value = to_address(0u8, public_key_hash, &mut bytes)?;
      let fields = [amount_field, Field { name: "Address", value }];
      let mut review_message_bytes = [0u8; 26]; // b"Review Output #".len() + 11
      let review_message = num_with_prefix(b"Review Output #", &I32::unsafe_from(current_index), &mut review_message_bytes)?;
      // TODO: display tokens
      return display(&fields, review_message);
    },
    LockupScript::Unknown => return Err(ErrorCode::NotSupported),
  }
}

fn display<'a>(fields: &'a [Field<'a>], review_message: &str) -> Result<(), ErrorCode> {
  let review_messages = [review_message];
  let review = MultiFieldReview::new(
    fields,
    &review_messages,
    Some(&EYE),
    "Approve",
    Some(&CHECKMARK),
    "Reject",
    Some(&CROSS),
  );
  if review.show() { Ok(()) } else { Err(ErrorCode::UserCancelled) }
}
