use crate::{
    blake2b_hasher::Blake2bHasher,
    error_code::ErrorCode,
    ledger_sdk_stub::{nvm::{NVMData, NVM, NVM_DATA_SIZE}, swapping_buffer::{SwappingBuffer, RAM_SIZE}},
    public_key::{derive_pub_key_by_path, hash_of_public_key},
};
use core::str::from_utf8;
#[cfg(not(any(target_os = "stax", target_os = "flex")))]
use ledger_device_sdk::ui::{bitmaps::{EYE, CHECKMARK, CROSS, WARNING}, gadgets::Field};
#[cfg(not(any(target_os = "stax", target_os = "flex")))]
use crate::ledger_sdk_stub::multi_field_review::MultiFieldReview;
#[cfg(any(target_os = "stax", target_os = "flex"))]
use ledger_device_sdk::nbgl::{Field, TagValueList};
#[cfg(any(target_os = "stax", target_os = "flex"))]
use crate::nbgl::{nbgl_review_fields, nbgl_sync_review_status, ReviewType};
use utils::{
    base58::{base58_encode_inputs, ALPHABET},
    types::{AssetOutput, Byte32, LockupScript, TxInput, UnlockScript, UnsignedTx, I32, U256},
};

#[link_section = ".nvm_data"]
static mut DATA: NVMData<NVM<NVM_DATA_SIZE>> = NVMData::new(NVM::zeroed());

pub struct TxReviewer {
    buffer: SwappingBuffer<'static, RAM_SIZE, NVM_DATA_SIZE>,
    has_external_inputs: bool,
    next_output_index: u16,
    tx_fee: Option<U256>,
}

impl TxReviewer {
    pub fn new() -> Self {
        Self {
            buffer: unsafe { SwappingBuffer::new(&mut DATA) },
            has_external_inputs: false,
            next_output_index: 1, // display output from index 1, similar to BTC
            tx_fee: None,
        }
    }

    #[inline]
    fn reset_buffer(&mut self) {
        self.buffer.reset();
    }

    #[inline]
    pub fn reset(&mut self) {
        self.reset_buffer();
        self.has_external_inputs = false;
        self.next_output_index = 1;
        self.tx_fee = None;
    }

    fn write_alph_amount(&mut self, u256: &U256) -> Result<usize, ErrorCode> {
        let mut amount_output = [0u8; 33];
        let amount_str = u256.to_alph(&mut amount_output).unwrap();
        self.buffer.write(amount_str)
    }

    fn write_token_amount(&mut self, u256: &U256) -> Result<usize, ErrorCode> {
        let mut amount_output = [0u8; 78]; // u256 max
        let amount_str = u256.to_str(&mut amount_output).unwrap();
        self.buffer.write(amount_str)
    }

    fn write_token_id(&mut self, token_id: &Byte32) -> Result<usize, ErrorCode> {
        let hex_str: [u8; 64] = utils::to_hex(&token_id.0).unwrap();
        self.buffer.write(&hex_str)
    }

    fn update_with_carry(
        &mut self,
        from: usize,
        to: usize,
        carry: usize,
    ) -> Result<usize, ErrorCode> {
        let mut bytes = [0u8; 64];
        let mut from_index = from;
        let mut new_carry = carry;
        while from_index < to {
            let stored = self.buffer.read(from_index, from_index + 64);
            for index in 0..64 {
                new_carry += (stored[index] as usize) << 8;
                bytes[index] = (new_carry % 58) as u8;
                new_carry /= 58;
            }
            self.buffer.write_from(from_index, &bytes)?;
            bytes = [0; 64];
            from_index += 64;
        }
        Ok(new_carry)
    }

    fn finalize_multi_sig(&mut self, from: usize, to: usize) -> Result<(), ErrorCode> {
        let mut temp0 = [0u8; 64];
        let mut temp1 = [0u8; 64];
        let mut begin = from;
        let mut end = to;
        while begin < end {
            if (end - begin) <= 64 {
                let stored = self.buffer.read(begin, end);
                let length = end - begin;
                for i in 0..length {
                    temp0[length - i - 1] = ALPHABET[stored[i] as usize];
                }
                self.buffer.update(begin, &temp0[..length]);
                return Ok(());
            }

            let left = self.buffer.read(begin, begin + 64);
            let right = self.buffer.read(end - 64, end);
            for i in 0..64 {
                let index = 64 - i - 1;
                temp0[index] = ALPHABET[left[i] as usize];
                temp1[index] = ALPHABET[right[i] as usize];
            }
            self.buffer.update(begin, &temp1);
            self.buffer.update(end - 64, &temp0);
            end -= 64;
            begin += 64;
        }
        Ok(())
    }

    // This function only for multi-sig address, which has no leading zeros
    pub fn write_multi_sig(&mut self, input: &[u8]) -> Result<usize, ErrorCode> {
        let from_index = self.buffer.get_index();
        let mut output_length = 0;
        let mut output_index = 0;
        let mut output = [0u8; 64];

        for &val in input {
            let mut carry = val as usize;
            carry = self.update_with_carry(from_index, from_index + output_length, carry)?;

            for byte in &mut output[..(output_index - output_length)] {
                carry += (*byte as usize) << 8;
                *byte = (carry % 58) as u8;
                carry /= 58;
            }
            while carry > 0 {
                if (output_index - output_length) == output.len() {
                    self.buffer.write_from(from_index + output_length, &output)?;
                    output = [0u8; 64];
                    output_length += 64;
                }
                output[output_index - output_length] = (carry % 58) as u8;
                output_index += 1;
                carry /= 58;
            }
        }

        self.buffer.write_from(
            from_index + output_length,
            &output[..(output_index - output_length)],
        )?;
        let to_index = from_index + output_index;
        self.finalize_multi_sig(from_index, to_index)?;
        Ok(to_index)
    }

    fn write_index_with_prefix(&mut self, index: usize, prefix: &[u8]) -> Result<usize, ErrorCode> {
        let mut output = [0u8; 13];
        assert!(prefix.len() + 3 <= 13);
        output[..prefix.len()].copy_from_slice(prefix);
        let num_str_bytes = I32::unsafe_from(index).to_str(&mut output[prefix.len()..]);
        if num_str_bytes.is_none() {
            return Err(ErrorCode::Overflow);
        }
        let total_size = prefix.len() + num_str_bytes.unwrap().len();
        self.buffer.write(&output[..total_size])
    }

    pub fn write_address(&mut self, prefix: u8, hash: &[u8; 32]) -> Result<usize, ErrorCode> {
        let mut output = [0u8; 46];
        let str_bytes = to_base58_address(prefix, hash, &mut output)?;
        self.buffer.write(str_bytes)
    }

    fn prepare_output(
        &mut self,
        output: &AssetOutput,
        device_address: &DeviceAddress,
        temp_data: &[u8],
    ) -> Result<Option<OutputIndexes>, ErrorCode> {
        let address_from_index = self.buffer.get_index();
        let address_to_index = match &output.lockup_script {
            LockupScript::P2PKH(hash) | LockupScript::P2SH(hash) => {
                self.write_address(output.lockup_script.get_type(), &hash.0)?
            }
            LockupScript::P2MPKH(_) => self.write_multi_sig(temp_data)?,
            _ => panic!(), // dead branch
        };

        let address = self.buffer.read(address_from_index, address_to_index);
        if device_address.eq(address) {
            return Ok(None);
        }

        let review_message_from_index = self.buffer.get_index();
        let review_message_to_index = self.write_index_with_prefix(self.next_output_index as usize, b"Output #")?;
        self.next_output_index += 1;

        let alph_amount_from_index = self.buffer.get_index();
        let alph_amount_to_index = self.write_alph_amount(&output.amount)?;

        let output_indexes = OutputIndexes {
            review_message: (review_message_from_index, review_message_to_index),
            alph_amount: (alph_amount_from_index, alph_amount_to_index),
            address: (address_from_index, address_to_index),
            token: None,
        };
        if output.tokens.is_empty() {
            return Ok(Some(output_indexes));
        }

        // Asset output has at most one token
        let token = output.tokens.get_current_item().unwrap();
        let token_id_from_index = self.buffer.get_index();
        let token_id_to_index = self.write_token_id(&token.id)?;

        let token_amount_from_index = self.buffer.get_index();
        let token_amount_to_index = self.write_token_amount(&token.amount)?;

        Ok(Some(OutputIndexes {
            token: Some(TokenIndexes {
                token_id: (token_id_from_index, token_id_to_index),
                token_amount: (token_amount_from_index, token_amount_to_index),
            }),
            ..output_indexes
        }))
    }

    fn get_str_from_range(&self, range: (usize, usize)) -> Result<&str, ErrorCode> {
        let bytes = self.buffer.read(range.0, range.1);
        bytes_to_string(bytes)
    }

    pub fn review_input(
        &mut self,
        input: &TxInput,
        current_index: usize,
        input_size: usize,
        device_address: &DeviceAddress
    ) -> Result<(), ErrorCode> {
        assert!(current_index < input_size);
        match &input.unlock_script {
            UnlockScript::P2PKH(public_key) => {
                let mut address_bytes = [0u8; 46];
                let public_key_hash = Blake2bHasher::hash(&public_key.0)?;
                let address = to_base58_address(0u8, &public_key_hash, &mut address_bytes)?;
                if !self.has_external_inputs {
                    self.has_external_inputs = !device_address.eq(address)
                }
            }
            UnlockScript::P2MPKH(_) => self.has_external_inputs = true,
            UnlockScript::P2SH(_) => self.has_external_inputs = true,
            UnlockScript::SameAsPrevious => (),
            _ => panic!(),
        };

        if (current_index == input_size - 1) && self.has_external_inputs {
            warning_external_inputs()?;
        }
        Ok(())
    }

    pub fn review_output(
        &mut self,
        output: &AssetOutput,
        device_address: &DeviceAddress,
        temp_data: &[u8],
    ) -> Result<(), ErrorCode> {
        let output_indexes_opt = self.prepare_output(output, device_address, temp_data)?;
        if output_indexes_opt.is_none() {
            return Ok(());
        }
        let OutputIndexes {
            review_message,
            alph_amount,
            address,
            token,
        } = output_indexes_opt.unwrap();
        let review_message = self.get_str_from_range(review_message)?;
        let alph_amount = self.get_str_from_range(alph_amount)?;
        let address = self.get_str_from_range(address)?;
        let address_field = Field {
            name: "Address",
            value: address,
        };
        let alph_amount_field = Field {
            name: "ALPH",
            value: alph_amount,
        };
        if token.is_none() {
            let fields = [address_field, alph_amount_field];
            return review(&fields, review_message)
        }

        let TokenIndexes {
            token_id,
            token_amount,
        } = token.unwrap();
        let token_id = self.get_str_from_range(token_id)?;
        let token_amount = self.get_str_from_range(token_amount)?;
        let fields = [
            address_field,
            alph_amount_field,
            Field {
                name: "Token ID",
                value: token_id,
            },
            Field {
                name: "Token Amount",
                value: token_amount,
            },
        ];
        review(&fields, review_message)
    }

    pub fn review_tx_details(
        &mut self,
        unsigned_tx: &UnsignedTx,
        device_address: &DeviceAddress,
        temp_data: &SwappingBuffer<'static, RAM_SIZE, NVM_DATA_SIZE>,
    ) -> Result<(), ErrorCode> {
        match unsigned_tx {
            UnsignedTx::NetworkId(_) => Ok(()),
            UnsignedTx::TxFee(tx_fee) => {
                let fee = tx_fee.inner.get();
                if fee.is_none() {
                    return Err(ErrorCode::Overflow);
                }
                self.tx_fee = Some(fee.as_ref().unwrap().clone());
                Ok(())
            },
            UnsignedTx::Inputs(inputs) => {
                if let Some(current_input) = inputs.get_current_item() {
                    self.review_input(
                        current_input,
                        inputs.current_index as usize,
                        inputs.size(),
                        device_address,
                    )
                } else {
                    Ok(())
                }
            }
            UnsignedTx::FixedOutputs(outputs) => {
                if let Some(current_output) = outputs.get_current_item() {
                    let result = self.review_output(
                        current_output,
                        device_address,
                        temp_data.read_all(),
                    );
                    self.reset_buffer();
                    if result.is_ok() && (outputs.current_index as usize == outputs.size() - 1) {
                        self.review_tx_fee()
                    } else {
                        result
                    }
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    pub fn review_tx_fee(&self) -> Result<(), ErrorCode> {
        assert!(self.tx_fee.is_some());
        let mut amount_output = [0u8; 33];
        let amount_str = self.tx_fee.as_ref().unwrap().to_alph(&mut amount_output).unwrap();
        let value = bytes_to_string(amount_str)?;
        let fields = [Field {
            name: "Fees",
            value,
        }];
        #[cfg(not(any(target_os = "stax", target_os = "flex")))]
        {
            let review = MultiFieldReview::new(
                &fields,
               &[],
               None,
               "Sign transaction",
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

        #[cfg(any(target_os = "stax", target_os = "flex"))]
        {
            let result = review(&fields, "Fees");
            if result.is_ok() {
                nbgl_sync_review_status(ReviewType::Transaction);
            }
            result
        }
    }
}

pub struct OutputIndexes {
    pub review_message: (usize, usize),
    pub alph_amount: (usize, usize),
    pub address: (usize, usize),
    pub token: Option<TokenIndexes>,
}

pub struct TokenIndexes {
    pub token_id: (usize, usize),
    pub token_amount: (usize, usize),
}

#[inline]
fn bytes_to_string(bytes: &[u8]) -> Result<&str, ErrorCode> {
    from_utf8(bytes).map_err(|_| ErrorCode::InternalError)
}

fn review<'a>(fields: &'a [Field<'a>], review_message: &str) -> Result<(), ErrorCode> {
    #[cfg(not(any(target_os = "stax", target_os = "flex")))]
    {
        let review_messages = ["Review ", review_message];
        let review = MultiFieldReview::new(
            fields,
           &review_messages,
           Some(&EYE),
           "Continue",
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

    #[cfg(any(target_os = "stax", target_os = "flex"))]
    {
        let values = TagValueList::new(fields, 0, false, false);
        let approved = nbgl_review_fields("Review", review_message, &values);
        if approved {
            Ok(())
        } else {
            Err(ErrorCode::UserCancelled)
        }
    }
}

fn warning_external_inputs() -> Result<(), ErrorCode> {
    #[cfg(not(any(target_os = "stax", target_os = "flex")))]
    {
        let review_messages = ["There are ", "external inputs"];
        let review = MultiFieldReview::new(
           &[],
           &review_messages,
           Some(&WARNING),
           "Continue",
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

    #[cfg(any(target_os = "stax", target_os = "flex"))]
    {
        // TODO: support stax & flex
        Ok(())
    }
}

#[inline]
pub fn to_base58_address<'a>(
    prefix: u8,
    hash: &[u8; 32],
    output: &'a mut [u8],
) -> Result<&'a [u8], ErrorCode> {
    if let Some(str_bytes) = base58_encode_inputs(&[&[prefix], &hash[..]], output) {
        Ok(str_bytes)
    } else {
        Err(ErrorCode::Overflow)
    }
}

pub struct DeviceAddress {
    bytes: [u8; 46],
    length: usize
}

impl DeviceAddress {
    pub fn from_path(path: &[u32]) -> Result<Self, ErrorCode> {
        let mut bytes = [0u8; 46];
        let device_public_key =
            derive_pub_key_by_path(path).map_err(|_| ErrorCode::DerivingPublicKeyFailed)?;
        let public_key_hash = hash_of_public_key(device_public_key.as_ref());
        let device_address = to_base58_address(0u8, &public_key_hash, &mut bytes)?;
        let length = device_address.len();
        Ok(Self { bytes, length })
    }

    pub fn eq(&self, addr: &[u8]) -> bool {
        &self.bytes[..self.length] == addr
    }
}
