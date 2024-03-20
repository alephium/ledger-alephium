use ledger_device_sdk::ecc::Secp256k1;
use ledger_device_sdk::ecc::SeedDerive;
use ledger_device_sdk::io::ApduHeader;
use ledger_device_sdk::ui::gadgets::MessageScroller;
use utils::{buffer::Buffer, decode::PartialDecoder, deserialize_path, types::UnsignedTx};

use crate::blind_signing::is_blind_signing_enabled;
use crate::nvm_buffer::NVMData;
use crate::nvm_buffer::NvmBuffer;
use crate::nvm_buffer::NVM;
use crate::tx_reviewer::TxReviewer;
use crate::{
    blake2b_hasher::{Blake2bHasher, BLAKE2B_HASH_SIZE},
    error_code::ErrorCode,
};

const SIZE: usize = 2048;

#[link_section = ".nvm_data"]
static mut DATA: NVMData<NVM<SIZE>> = NVMData::new(NVM::zeroed());

#[derive(PartialEq)]
enum DecodeStep {
    Init,
    DecodingTx,
    Complete,
}

pub struct SignTxContext {
    pub path: [u32; 5],
    unsigned_tx: PartialDecoder<UnsignedTx>,
    current_step: DecodeStep,
    hasher: Blake2bHasher,
    temp_data: NvmBuffer<'static, SIZE>,
}

impl SignTxContext {
    pub fn new() -> Self {
        SignTxContext {
            path: [0; 5],
            unsigned_tx: PartialDecoder::default(),
            current_step: DecodeStep::Init,
            hasher: Blake2bHasher::new(),
            temp_data: unsafe { NvmBuffer::new(&mut DATA) },
        }
    }

    pub fn reset(&mut self) {
        self.path = [0; 5];
        self.unsigned_tx.reset();
        self.current_step = DecodeStep::Init;
        self.hasher.reset();
        self.temp_data.reset();
    }

    pub fn is_complete(&self) -> bool {
        self.current_step == DecodeStep::Complete
    }

    pub fn get_tx_id(&mut self) -> Result<[u8; BLAKE2B_HASH_SIZE], ErrorCode> {
        self.hasher.finalize()
    }

    fn get_temp_data(&self) -> Result<&[u8], ErrorCode> {
        if self.temp_data.is_overflow() {
            return Err(ErrorCode::Overflow);
        }
        return Ok(self.temp_data.read());
    }

    fn review(&mut self, tx_reviewer: &mut TxReviewer) -> Result<(), ErrorCode> {
        match &self.unsigned_tx.inner {
            UnsignedTx::NetworkId(byte) => TxReviewer::review_network(byte.0),
            UnsignedTx::GasAmount(amount) => TxReviewer::review_gas_amount(amount),
            UnsignedTx::GasPrice(amount) => tx_reviewer.review_gas_price(amount),
            UnsignedTx::Inputs(inputs) => {
                let current_input = inputs.get_current_item();
                if current_input.is_some() {
                    tx_reviewer.review_input(
                        current_input.unwrap(),
                        inputs.current_index as usize,
                        self.get_temp_data()?,
                    )
                } else {
                    Ok(())
                }
            }
            UnsignedTx::FixedOutputs(outputs) => {
                let current_output = outputs.get_current_item();
                if current_output.is_some() {
                    tx_reviewer.review_output(
                        current_output.unwrap(),
                        outputs.current_index as usize,
                        self.get_temp_data()?,
                    )
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    pub fn review_tx_id_and_sign(&mut self) -> Result<([u8; 72], u32, u32), ErrorCode> {
        let tx_id = self.get_tx_id()?;
        TxReviewer::review_tx_id(&tx_id)?;
        let signature = Secp256k1::derive_from_path(&self.path)
            .deterministic_sign(&tx_id)
            .map_err(|_| ErrorCode::TxSignFail)?;
        Ok(signature)
    }

    fn _decode_tx<'a>(
        &mut self,
        buffer: &mut Buffer<'a, NvmBuffer<'static, SIZE>>,
        tx_reviewer: &mut TxReviewer,
    ) -> Result<(), ErrorCode> {
        while !buffer.is_empty() {
            match self.unsigned_tx.try_decode_one_step(buffer) {
                Ok(true) => {
                    self.review(tx_reviewer)?;
                    self.temp_data.reset();
                    tx_reviewer.reset();
                    if self.unsigned_tx.inner.is_complete() {
                        self.current_step = DecodeStep::Complete;
                        return Ok(());
                    } else {
                        self.unsigned_tx.inner.next_step();
                        self.unsigned_tx.reset_stage();
                    }
                }
                Ok(false) => return Ok(()),
                Err(_) => return Err(ErrorCode::TxDecodeFail),
            }
        }
        Ok(())
    }

    fn decode_tx(&mut self, data: &[u8], tx_reviewer: &mut TxReviewer) -> Result<(), ErrorCode> {
        if data.len() > (u8::MAX as usize) {
            return Err(ErrorCode::BadLen);
        }
        let mut buffer = Buffer::new(data, &mut self.temp_data).unwrap();
        let from_index = buffer.get_index();
        let result = self._decode_tx(&mut buffer, tx_reviewer);
        let to_index = buffer.get_index();
        self.hasher.update(buffer.get_range(from_index, to_index))?;
        result
    }

    pub fn handle_data(
        &mut self,
        apdu_header: &ApduHeader,
        data: &[u8],
        tx_reviewer: &mut TxReviewer,
    ) -> Result<(), ErrorCode> {
        match self.current_step {
            DecodeStep::Complete => Err(ErrorCode::InternalError),
            DecodeStep::Init => {
                if apdu_header.p1 == 0 && data.len() >= 23 {
                    if !deserialize_path(&data[0..20], &mut self.path) {
                        return Err(ErrorCode::DerivePathDecodeFail);
                    }
                    self.current_step = DecodeStep::DecodingTx;
                    let tx_data = &data[20..];
                    if tx_data[2] == 0x01 {
                        check_blind_signing()?;
                    }
                    self.decode_tx(tx_data, tx_reviewer)
                } else {
                    Err(ErrorCode::BadLen)
                }
            }
            _ => {
                if apdu_header.p1 == 1 {
                    self.decode_tx(data, tx_reviewer)
                } else {
                    Err(ErrorCode::BadP1P2)
                }
            }
        }
    }
}

fn check_blind_signing() -> Result<(), ErrorCode> {
    if is_blind_signing_enabled() {
        return Ok(());
    }
    let scroller = MessageScroller::new("Blind signing must be enabled");
    scroller.event_loop();
    Err(ErrorCode::BlindSigningNotEnabled)
}
