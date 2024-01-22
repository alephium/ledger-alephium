use ledger_device_sdk::io::ApduHeader;
use utils::{decode::PartialDecoder, types::UnsignedTx, deserialize_path, buffer::Buffer};

use crate::{print, blake2b_hasher::{Blake2bHasher, BLAKE2B_HASH_SIZE}, error_code::ErrorCode};

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

  pub fn is_complete(&self) -> bool {
    self.current_step == DecodeStep::Complete
  }

  pub fn get_tx_id(&mut self) -> Result<[u8; BLAKE2B_HASH_SIZE], ErrorCode> {
    self.hasher.finalize()
  }

  fn decode_path(&mut self, data: &[u8]) {
    deserialize_path(data, &mut self.path);
    self.current_step = DecodeStep::TxVersion;
  }

  fn display(&mut self) {
    match self.current_step {
      DecodeStep::TxVersion => {
        print::println_slice::<1>(&[self.unsigned_tx.inner.version.0]);
        self.current_step = DecodeStep::TxNetworkId;
      }
      DecodeStep::TxNetworkId => {
        print::println_slice::<1>(&[self.unsigned_tx.inner.network_id.0]);
        self.current_step = DecodeStep::TxScriptOpt;
      }
      DecodeStep::TxScriptOpt => {
        self.current_step = DecodeStep::TxGasAmount;
      },
      DecodeStep::TxGasAmount => {
        print::println_slice::<4>(&self.unsigned_tx.inner.gas_amount.inner.to_be_bytes());
        self.current_step = DecodeStep::TxGasPrice;
      },
      DecodeStep::TxGasPrice => {
        print::println_slice::<8>(&self.unsigned_tx.inner.gas_price.inner[0].to_be_bytes());
        print::println_slice::<8>(&self.unsigned_tx.inner.gas_price.inner[1].to_be_bytes());
        print::println_slice::<8>(&self.unsigned_tx.inner.gas_price.inner[2].to_be_bytes());
        print::println_slice::<8>(&self.unsigned_tx.inner.gas_price.inner[3].to_be_bytes());
        self.current_step = DecodeStep::TxInputs;
      },
      DecodeStep::TxInputs => {
        print::println_slice::<4>(&self.unsigned_tx.inner.inputs.inner.size().to_be_bytes());
        print::println_slice::<4>(&self.unsigned_tx.inner.inputs.inner.current_index.to_be_bytes());
        if self.unsigned_tx.inner.inputs.inner.is_complete() {
          self.current_step = DecodeStep::TxOutputs;
        }
      },
      DecodeStep::TxOutputs => {
        print::println_slice::<4>(&self.unsigned_tx.inner.fixed_outputs.inner.size().to_be_bytes());
        print::println_slice::<4>(&self.unsigned_tx.inner.fixed_outputs.inner.current_index.to_be_bytes());
        if self.unsigned_tx.inner.fixed_outputs.inner.is_complete() {
          self.current_step = DecodeStep::Complete;
        }
      },
      _ => ()
    }
  }

  fn _decode_tx(&mut self, buffer: &mut Buffer) -> Result<(), ErrorCode> {
    match self.unsigned_tx.try_decode_one_step(buffer) {
      Ok(true) => {
        self.display();
        if self.unsigned_tx.stage.is_complete() {
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
          self.current_step = DecodeStep::DerivePath;
          self.decode_path(&data[0..20]);
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
