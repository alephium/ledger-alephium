use core::str::from_utf8;
use ledger_device_sdk::ecc::Secp256k1;
use ledger_device_sdk::ecc::SeedDerive;
use ledger_device_sdk::io::ApduHeader;
use ledger_device_sdk::ui::bitmaps::{CHECKMARK, CROSS, EYE};
use ledger_device_sdk::ui::gadgets::{Field, MessageScroller, MultiFieldReview};
use ledger_device_sdk::Pic;
use utils::base58::base58_encode_inputs;
use utils::types::lockup_script::P2MPKH;
use utils::types::{AssetOutput, LockupScript, TxInput, UnlockScript, I32, U256};
use utils::{buffer::Buffer, decode::PartialDecoder, deserialize_path, types::UnsignedTx};

use crate::blind_signing::is_blind_signing_enabled;
use crate::nvm_buffer::NvmBuffer;
use crate::nvm_buffer::NVM;
use crate::{
    blake2b_hasher::{Blake2bHasher, BLAKE2B_HASH_SIZE},
    error_code::ErrorCode,
};

const SIZE: usize = 2048;

#[link_section = ".rodata.N_"]
static mut DATA: Pic<NVM<SIZE>> = Pic::new(NVM::zeroed());

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

    fn get_temp_data(&self) -> Option<&[u8]> {
        if self.temp_data.is_overflow() {
            return None;
        }
        return Some(self.temp_data.read());
    }

    fn review(&mut self) -> Result<(), ErrorCode> {
        match &self.unsigned_tx.inner {
            UnsignedTx::NetworkId(byte) => review_network(byte.0),
            UnsignedTx::GasAmount(amount) => review_gas_amount(amount),
            UnsignedTx::GasPrice(amount) => review_gas_price(amount),
            UnsignedTx::Inputs(inputs) => {
                let current_input = inputs.get_current_item();
                if current_input.is_some() {
                    review_tx_input(
                        current_input.unwrap(),
                        inputs.current_index as usize,
                        self.get_temp_data(),
                    )
                } else {
                    Ok(())
                }
            }
            UnsignedTx::FixedOutputs(outputs) => {
                let current_output = outputs.get_current_item();
                if current_output.is_some() {
                    let output = current_output.unwrap();
                    review_tx_output(output, outputs.current_index as usize, self.get_temp_data())?;
                    review_token(output)
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    pub fn review_tx_id_and_sign(&mut self) -> Result<([u8; 72], u32, u32), ErrorCode> {
        let tx_id = self.get_tx_id()?;
        let hex: [u8; 64] = utils::to_hex(&tx_id).unwrap();
        let hex_str = bytes_to_string(&hex)?;
        let fields = [Field {
            name: "TxId",
            value: hex_str,
        }];
        review(&fields, "Review Tx Id")?;
        let signature = Secp256k1::derive_from_path(&self.path)
            .deterministic_sign(&tx_id)
            .map_err(|_| ErrorCode::TxSignFail)?;
        Ok(signature)
    }

    fn _decode_tx<'a>(
        &mut self,
        buffer: &mut Buffer<'a, NvmBuffer<'static, SIZE>>,
    ) -> Result<(), ErrorCode> {
        while !buffer.is_empty() {
            match self.unsigned_tx.try_decode_one_step(buffer) {
                Ok(true) => {
                    self.review()?;
                    if self.unsigned_tx.inner.is_complete() {
                        self.current_step = DecodeStep::Complete;
                        return Ok(());
                    } else {
                        self.unsigned_tx.inner.next_step();
                        self.unsigned_tx.reset_stage();
                        self.temp_data.reset();
                    }
                }
                Ok(false) => return Ok(()),
                Err(_) => return Err(ErrorCode::TxDecodeFail),
            }
        }
        Ok(())
    }

    fn decode_tx(&mut self, data: &[u8]) -> Result<(), ErrorCode> {
        if data.len() > (u8::MAX as usize) {
            return Err(ErrorCode::BadLen);
        }
        let mut buffer = Buffer::new(data, &mut self.temp_data).unwrap();
        let from_index = buffer.get_index();
        let result = self._decode_tx(&mut buffer);
        let to_index = buffer.get_index();
        self.hasher.update(buffer.get_range(from_index, to_index))?;
        result
    }

    pub fn handle_data(&mut self, apdu_header: &ApduHeader, data: &[u8]) -> Result<(), ErrorCode> {
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
                    self.decode_tx(tx_data)
                } else {
                    Err(ErrorCode::BadLen)
                }
            }
            _ => {
                if apdu_header.p1 == 1 {
                    self.decode_tx(data)
                } else {
                    Err(ErrorCode::BadP1P2)
                }
            }
        }
    }
}

fn review_network(id: u8) -> Result<(), ErrorCode> {
    let network_type = match id {
        0 => "mainnet",
        1 => "testnet",
        _ => "devnet",
    };

    let fields = [Field {
        name: "Network",
        value: network_type,
    }];
    review(&fields, "Review Network")
}

#[inline]
fn bytes_to_string(bytes: &[u8]) -> Result<&str, ErrorCode> {
    from_utf8(bytes).map_err(|_| ErrorCode::InternalError)
}

fn index_with_prefix<'a>(
    prefix: &[u8],
    num: &I32,
    output: &'a mut [u8],
) -> Result<&'a str, ErrorCode> {
    let mut num_output: [u8; 3] = output[prefix.len()..]
        .try_into()
        .map_err(|_| ErrorCode::Overflow)?;
    let num_str_bytes = num.to_str(&mut num_output);
    if num_str_bytes.is_none() {
        return Err(ErrorCode::Overflow);
    }
    output[..prefix.len()].copy_from_slice(prefix);
    let num_str = num_str_bytes.unwrap();
    let total_size = prefix.len() + num_str.len();
    output[prefix.len()..total_size].copy_from_slice(num_str);
    bytes_to_string(&output[..total_size])
}

fn to_alph_str<'a>(amount: &U256, output: &'a mut [u8]) -> Result<&'a str, ErrorCode> {
    let str_bytes = amount.to_alph(output);
    if str_bytes.is_none() {
        return Err(ErrorCode::Overflow);
    }
    bytes_to_string(&str_bytes.unwrap())
}

fn to_address<'a, const NUM: usize>(
    prefix: u8,
    hash: &[u8; 32],
    output: &'a mut [u8; NUM],
) -> Result<&'a str, ErrorCode> {
    let str_bytes = base58_encode_inputs(&[&[prefix], hash], output);
    if str_bytes.is_none() {
        return Err(ErrorCode::Overflow);
    }
    bytes_to_string(str_bytes.unwrap())
}

fn review_gas_amount(gas_amount: &I32) -> Result<(), ErrorCode> {
    let mut output = [0; 11];
    let num_str_bytes = gas_amount.to_str(&mut output);
    if num_str_bytes.is_none() {
        return Err(ErrorCode::Overflow);
    }
    let value = bytes_to_string(&num_str_bytes.unwrap())?;
    let fields = [Field {
        name: "GasAmount",
        value,
    }];
    review(&fields, "Review Gas Amount")
}

fn review_gas_price(gas_price: &U256) -> Result<(), ErrorCode> {
    let mut output = [0; 33];
    let value = to_alph_str(gas_price, &mut output)?;
    let fields = [Field {
        name: "GasPrice",
        value,
    }];
    review(&fields, "Review Gas Price")
}

fn review_tx_input(
    tx_input: &TxInput,
    current_index: usize,
    temp_data: Option<&[u8]>,
) -> Result<(), ErrorCode> {
    let mut review_message_bytes = [0u8; 17]; // b"Review Input #".len() + 3
    let review_message = index_with_prefix(
        b"Review Input #",
        &I32::unsafe_from(current_index),
        &mut review_message_bytes,
    )?;
    match &tx_input.unlock_script {
        UnlockScript::P2PKH(public_key) => {
            let public_key_hash = Blake2bHasher::hash(&public_key.0)?;
            let mut bytes = [0u8; 46];
            let value = to_address(0u8, &public_key_hash, &mut bytes)?;
            let fields = [Field {
                name: "Address",
                value,
            }];
            review(&fields, review_message)
        }
        UnlockScript::P2MPKH(_) => {
            check_blind_signing()?;
            let fields = [Field {
                name: "Address",
                value: "multi-sig address",
            }];
            review(&fields, review_message)
        }
        UnlockScript::P2SH(_) => {
            let default_value = "p2sh address";
            let mut bytes = [0u8; 46];
            let address = if temp_data.is_some() {
                let script_bytes = temp_data.unwrap();
                let script_hash = Blake2bHasher::hash(script_bytes)?;
                to_address(2u8, &script_hash, &mut bytes)?
            } else {
                check_blind_signing()?;
                default_value
            };
            let fields = [Field {
                name: "Address",
                value: address,
            }];
            review(&fields, review_message)
        }
        UnlockScript::SameAsPrevious => {
            let fields = [Field {
                name: "Address",
                value: "same as previous",
            }];
            review(&fields, review_message)
        }
        _ => Err(ErrorCode::InternalError),
    }
}

fn review_tx_output(
    output: &AssetOutput,
    current_index: usize,
    temp_data: Option<&[u8]>,
) -> Result<(), ErrorCode> {
    let mut amount_output = [0u8; 33];
    let amount_str = to_alph_str(&output.amount, &mut amount_output)?;
    let amount_field = Field {
        name: "Amount",
        value: amount_str,
    };
    let mut review_message_bytes = [0u8; 18]; // b"Review Output #".len() + 3
    let review_message = index_with_prefix(
        b"Review Output #",
        &I32::unsafe_from(current_index),
        &mut review_message_bytes,
    )?;
    match &output.lockup_script {
        LockupScript::P2PKH(hash) | LockupScript::P2SH(hash) => {
            let mut bytes = [0u8; 46];
            let value = to_address(output.lockup_script.get_type(), &hash.0, &mut bytes)?;
            let fields = [
                amount_field,
                Field {
                    name: "Address",
                    value,
                },
            ];
            review(&fields, review_message)
        }
        LockupScript::P2MPKH(p2mpkh) => {
            let mut bs58_str = [0u8; P2MPKH::BASE58_OUTPUT_SIZE];
            let default_address = "multi-sig address";
            let address = if p2mpkh.inner.is_reviewable() && temp_data.is_some() {
                let encoded = temp_data.unwrap();
                let prefix = [0x01u8, p2mpkh.inner.size.inner as u8];
                let postfix = [p2mpkh.inner.m.inner as u8];
                let result = base58_encode_inputs(&[&prefix, encoded, &postfix], &mut bs58_str);
                if result.is_none() {
                    check_blind_signing()?;
                    default_address
                } else {
                    bytes_to_string(result.unwrap())?
                }
            } else {
                check_blind_signing()?;
                default_address
            };
            let fields = [
                amount_field,
                Field {
                    name: "Address",
                    value: address,
                },
            ];
            review(&fields, review_message)
        }
        _ => Err(ErrorCode::InternalError),
    }
}

fn review_token(output: &AssetOutput) -> Result<(), ErrorCode> {
    if output.tokens.is_empty() {
        return Ok(());
    }
    let token_opt = output.tokens.get_current_item();
    if token_opt.is_none() {
        return Ok(());
    }
    let token = token_opt.unwrap();
    let mut amount_output = [0u8; 39]; // u128 max
    let amount_str_bytes = token.amount.to_str(&mut amount_output);
    let amount_str = if amount_str_bytes.is_none() {
        check_blind_signing()?;
        "number too large"
    } else {
        bytes_to_string(&amount_str_bytes.unwrap())?
    };
    let token_id_bytes: [u8; 64] = utils::to_hex(&token.id.0).unwrap();
    let token_id_hex = bytes_to_string(&token_id_bytes)?;
    let fields = [
        Field {
            name: "Token Id",
            value: token_id_hex,
        },
        Field {
            name: "Token Amount",
            value: amount_str,
        },
    ];
    review(&fields, "Review Output Token")
}

fn review<'a>(fields: &'a [Field<'a>], review_message: &str) -> Result<(), ErrorCode> {
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
    if review.show() {
        Ok(())
    } else {
        Err(ErrorCode::UserCancelled)
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
