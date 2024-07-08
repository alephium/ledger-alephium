use ledger_device_sdk::ecc::Secp256k1;
use ledger_device_sdk::ecc::SeedDerive;
use ledger_device_sdk::io::ApduHeader;
use ledger_device_sdk::ui::gadgets::MessageScroller;
use utils::{buffer::Buffer, decode::PartialDecoder, deserialize_path, types::UnsignedTx};

use crate::blind_signing::is_blind_signing_enabled;
use crate::nvm_buffer::{NVMData, NVMBuffer, NVM, NVM_DATA_SIZE};
use crate::tx_reviewer::TxReviewer;
use crate::{
    blake2b_hasher::{Blake2bHasher, BLAKE2B_HASH_SIZE},
    error_code::ErrorCode,
};

#[link_section = ".nvm_data"]
static mut DATA: NVMData<NVM<NVM_DATA_SIZE>> = NVMData::new(NVM::zeroed());

#[derive(PartialEq)]
enum DecodeStep {
    Init,
    DecodingTx,
    Complete,
}

pub struct SignTxContext {
    pub path: [u32; 5],
    tx_decoder: PartialDecoder<UnsignedTx>,
    current_step: DecodeStep,
    hasher: Blake2bHasher,
    temp_data: NVMBuffer<'static, NVM_DATA_SIZE>,
}

impl SignTxContext {
    pub fn new() -> Self {
        SignTxContext {
            path: [0; 5],
            tx_decoder: PartialDecoder::default(),
            current_step: DecodeStep::Init,
            hasher: Blake2bHasher::new(),
            temp_data: unsafe { NVMBuffer::new(&mut DATA) },
        }
    }

    pub fn reset(&mut self) {
        self.path = [0; 5];
        self.tx_decoder.reset();
        self.current_step = DecodeStep::Init;
        self.hasher.reset();
        self.temp_data.reset();
    }

    pub fn is_complete(&self) -> bool {
        self.current_step == DecodeStep::Complete
    }

    pub fn get_tx_id(&mut self) -> Result<[u8; BLAKE2B_HASH_SIZE], ErrorCode> {
        assert!(self.is_complete());
        self.hasher.finalize()
    }

    pub fn review_tx_id_and_sign(&mut self) -> Result<([u8; 72], u32, u32), ErrorCode> {
        let tx_id = self.get_tx_id()?;
        TxReviewer::review_tx_id(&tx_id)?;
        let signature = Secp256k1::derive_from_path(&self.path)
            .deterministic_sign(&tx_id)
            .map_err(|_| ErrorCode::TxSigningFailed)?;
        Ok(signature)
    }

    fn _decode_tx<'a>(
        &mut self,
        buffer: &mut Buffer<'a, NVMBuffer<'static, NVM_DATA_SIZE>>,
        tx_reviewer: &mut TxReviewer,
    ) -> Result<(), ErrorCode> {
        while !buffer.is_empty() {
            match self.tx_decoder.step(buffer) {
                Ok(true) => {
                    tx_reviewer.review_tx_details(&self.tx_decoder.inner, &self.temp_data)?;
                    self.temp_data.reset();
                    if self.tx_decoder.inner.is_complete() {
                        self.current_step = DecodeStep::Complete;
                        return Ok(());
                    } else {
                        self.tx_decoder.inner.next_step();
                        self.tx_decoder.reset_stage();
                    }
                }
                Ok(false) => return Ok(()),
                Err(_) => return Err(ErrorCode::TxDecodingFailed),
            }
        }
        Ok(())
    }

    fn decode_tx(&mut self, data: &[u8], tx_reviewer: &mut TxReviewer) -> Result<(), ErrorCode> {
        if data.len() > (u8::MAX as usize) {
            return Err(ErrorCode::BadLen);
        }
        let mut buffer = Buffer::new(data, &mut self.temp_data).unwrap();
        let result = self._decode_tx(&mut buffer, tx_reviewer);
        self.hasher.update(data)?;
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
                        return Err(ErrorCode::HDPathDecodingFailed);
                    }
                    self.current_step = DecodeStep::DecodingTx;
                    let tx_data = &data[20..];
                    if tx_data[2] == 0x01 { // if this tx calls contract
                        check_blind_signing()?;
                    }
                    self.decode_tx(tx_data, tx_reviewer)
                } else {
                    Err(ErrorCode::BadLen)
                }
            }
            DecodeStep::DecodingTx => {
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
    Err(ErrorCode::BlindSigningDisabled)
}
