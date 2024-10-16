use super::TxReviewerInner;
#[cfg(target_os = "nanos")]
use crate::nvm::nvm_data::NVMData;
use crate::{
    blake2b_hasher::Blake2bHasher,
    error_code::ErrorCode,
    handler::TOKEN_METADATA_SIZE,
    nvm::swapping_buffer::{SwappingBuffer, RAM_SIZE},
    nvm::{NVM, NVM_DATA_SIZE},
    public_key::{to_base58_address, Address},
    token_verifier::TokenVerifier,
    ui::bytes_to_string,
};
#[cfg(any(target_os = "stax", target_os = "flex"))]
use ledger_device_sdk::nbgl::Field;
#[cfg(not(any(target_os = "stax", target_os = "flex")))]
use ledger_device_sdk::ui::gadgets::Field;
#[cfg(not(target_os = "nanos"))]
use ledger_device_sdk::NVMData;
use utils::{
    base58::ALPHABET,
    types::{
        AssetOutput, Byte32, Hash, LockupScript, Token, TxInput, UnlockScript, UnsignedTx, I32,
        U256,
    },
};

#[link_section = ".nvm_data"]
static mut DATA: NVMData<NVM<NVM_DATA_SIZE>> = NVMData::new(NVM::zeroed());

const FIRST_OUTPUT_INDEX: u16 = 1;
const MAX_TOKEN_SYMBOL_LENGTH: usize = 12;
const TOKEN_METADATA_VERSION: u8 = 0;
type TokenSymbol = [u8; MAX_TOKEN_SYMBOL_LENGTH];

// The TxReviewer is used to review the transaction details
// It keeps track of the transaction details and the current state
// It also keeps track of the token metadata
pub struct TxReviewer {
    buffer: SwappingBuffer<'static, RAM_SIZE, NVM_DATA_SIZE>,
    has_external_inputs: bool,
    next_output_index: u16,
    tx_fee: Option<U256>,
    token_metadata_length: usize,
    token_verifier: Option<TokenVerifier>,
    inner: TxReviewerInner,
}

impl TxReviewer {
    pub fn new() -> Self {
        Self {
            buffer: unsafe { SwappingBuffer::new(&mut DATA) },
            has_external_inputs: false,
            next_output_index: FIRST_OUTPUT_INDEX, // display output from index 1, similar to BTC
            tx_fee: None,
            token_metadata_length: 0,
            token_verifier: None,
            inner: TxReviewerInner::new(),
        }
    }

    #[inline]
    fn reset_buffer(&mut self, from_index: usize) {
        self.buffer.reset(from_index);
    }

    #[inline]
    pub fn init(&mut self, token_size: u8) -> Result<(), ErrorCode> {
        self.reset_buffer(0);
        self.has_external_inputs = false;
        self.next_output_index = FIRST_OUTPUT_INDEX;
        self.tx_fee = None;
        self.token_metadata_length = (token_size as usize) * TOKEN_METADATA_SIZE;
        self.token_verifier = None;
        self.inner = TxReviewerInner::new();
        Ok(())
    }

    pub fn reset(&mut self) {
        self.reset_buffer(0);
        self.has_external_inputs = false;
        self.next_output_index = FIRST_OUTPUT_INDEX;
        self.tx_fee = None;
        self.token_metadata_length = 0;
        self.token_verifier = None;
        self.inner.reset();
    }

    pub fn handle_token_metadata(&mut self, data: &[u8]) -> Result<(), ErrorCode> {
        assert!(self.token_verifier.is_none());
        let token_verifier = TokenVerifier::new(data)?;
        // we have checked the data size in `TokenVerifier::new(data)`
        let token_metadata = &data[..TOKEN_METADATA_SIZE];
        if token_metadata[0] != TOKEN_METADATA_VERSION {
            // the first byte is the metadata version
            return Err(ErrorCode::InvalidMetadataVersion);
        }
        self.write_token_metadata(token_metadata)?;
        if !token_verifier.is_complete() {
            self.token_verifier = Some(token_verifier);
            return Ok(());
        }
        if token_verifier.is_token_valid() {
            Ok(())
        } else {
            Err(ErrorCode::InvalidTokenMetadata)
        }
    }

    pub fn handle_token_proof(&mut self, data: &[u8]) -> Result<(), ErrorCode> {
        assert!(self.token_verifier.is_some());
        let token_verifier = self.token_verifier.as_mut().unwrap();
        token_verifier.on_proof(data)?;
        if !token_verifier.is_complete() {
            return Ok(());
        }
        let result = if token_verifier.is_token_valid() {
            Ok(())
        } else {
            Err(ErrorCode::InvalidTokenMetadata)
        };
        self.token_verifier = None;
        result
    }

    fn write_token_metadata(&mut self, token_metadata: &[u8]) -> Result<(), ErrorCode> {
        let size = self.buffer.write(token_metadata)?;
        if size > self.token_metadata_length {
            Err(ErrorCode::InvalidTokenSize)
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn set_tx_execute_script(&mut self, is_tx_execute_script: bool) {
        self.inner.set_tx_execute_script(is_tx_execute_script);
    }

    // Write the amount in alph format
    fn write_alph_amount(&mut self, u256: &U256) -> Result<usize, ErrorCode> {
        let mut amount_output = [0u8; 33];
        let amount_str = u256.to_alph(&mut amount_output).unwrap();
        self.buffer.write(amount_str)
    }

    // Write the amount in raw format
    fn write_token_raw_amount(&mut self, u256: &U256) -> Result<usize, ErrorCode> {
        let mut amount_output = [0u8; 78]; // u256 max
        let amount_str = u256.to_str(&mut amount_output).unwrap();
        self.buffer.write(amount_str)
    }

    // Write the amount in token format
    fn write_token_amount(
        &mut self,
        u256: &U256,
        symbol: TokenSymbol,
        decimals: usize,
    ) -> Result<usize, ErrorCode> {
        let mut amount_output = [0u8; 86]; // u256 max
        let symbol_bytes = get_token_symbol_bytes(&symbol[..]);
        amount_output[..symbol_bytes.len()].copy_from_slice(symbol_bytes);
        amount_output[symbol_bytes.len()] = b' ';
        let prefix_length = symbol_bytes.len() + 1;
        let amount_str = u256.to_str_with_decimals(&mut amount_output[prefix_length..], decimals);
        if amount_str.is_none() {
            return Err(ErrorCode::Overflow);
        }
        let total_length = prefix_length + amount_str.unwrap().len();
        self.buffer.write(&amount_output[..total_length])
    }

    // Write the token id in hex format
    fn write_token_id(&mut self, token_id: &Byte32) -> Result<usize, ErrorCode> {
        let hex_str: [u8; 64] = utils::to_hex(&token_id.0).unwrap();
        self.buffer.write(&hex_str)
    }

    // Update the buffer with the carry
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

    // Finalize the multi-sig address
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
                    self.buffer
                        .write_from(from_index + output_length, &output)?;
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

    // Write the output index with a prefix
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

    // Write the address
    pub fn write_address(&mut self, prefix: u8, hash: &[u8; 32]) -> Result<usize, ErrorCode> {
        let mut output = [0u8; 46];
        let str_bytes = to_base58_address(prefix, hash, &mut output)?;
        self.buffer.write(str_bytes)
    }

    fn get_token_metadata(&self, token_id: &Hash) -> Option<(TokenSymbol, u8)> {
        let token_size = self.token_metadata_length / TOKEN_METADATA_SIZE;
        if token_size == 0 {
            return None;
        }
        for i in 0..token_size {
            let from_index = i * TOKEN_METADATA_SIZE;
            let to_index = from_index + TOKEN_METADATA_SIZE;
            let token_metadata_bytes = self.buffer.read(from_index, to_index);
            if token_metadata_bytes[1..33] == token_id.0 {
                let last_index = TOKEN_METADATA_SIZE - 1; // the last index of the encoded token metadata
                let token_symbol = token_metadata_bytes[33..last_index].try_into().unwrap();
                let token_decimals = token_metadata_bytes[last_index];
                return Some((token_symbol, token_decimals));
            }
        }
        None
    }

    // Prepare the output for review
    fn prepare_output(
        &mut self,
        output: &AssetOutput,
        device_address: &Address,
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
        let review_message_to_index =
            self.write_index_with_prefix(self.next_output_index as usize, b"Output #")?;
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
        let token_indexes = self.prepare_token(token)?;
        Ok(Some(OutputIndexes {
            token: Some(token_indexes),
            ..output_indexes
        }))
    }

    // Prepare the token for review
    fn prepare_token(&mut self, token: &Token) -> Result<TokenIndexes, ErrorCode> {
        let token_id_from_index = self.buffer.get_index();
        let token_id_to_index = self.write_token_id(&token.id)?;
        match self.get_token_metadata(&token.id) {
            Some((token_symbol, token_decimals)) => {
                let token_amount_from_index = self.buffer.get_index();
                let token_amount_to_index =
                    self.write_token_amount(&token.amount, token_symbol, token_decimals as usize)?;
                Ok(TokenIndexes {
                    has_token_metadata: true,
                    token_id: (token_id_from_index, token_id_to_index),
                    token_amount: (token_amount_from_index, token_amount_to_index),
                })
            }
            None => {
                let token_amount_from_index = self.buffer.get_index();
                let token_amount_to_index = self.write_token_raw_amount(&token.amount)?;
                Ok(TokenIndexes {
                    has_token_metadata: false,
                    token_id: (token_id_from_index, token_id_to_index),
                    token_amount: (token_amount_from_index, token_amount_to_index),
                })
            }
        }
    }

    fn get_str_from_range(&self, range: (usize, usize)) -> Result<&str, ErrorCode> {
        let bytes = self.buffer.read(range.0, range.1);
        bytes_to_string(bytes)
    }

    // Review the input for the transaction
    pub fn review_input(
        &mut self,
        input: &TxInput,
        current_index: usize,
        input_size: usize,
        device_address: &Address,
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
            self.inner.warning_external_inputs()?;
        }
        Ok(())
    }

    // Review the output for the transaction
    pub fn review_output(
        &mut self,
        output: &AssetOutput,
        device_address: &Address,
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
            name: "To",
            value: address,
        };
        let alph_amount_field = Field {
            name: "Amount",
            value: alph_amount,
        };
        let output_index_field = Field {
            name: "Transaction Output",
            value: review_message,
        };
        if token.is_none() {
            let all_fields = &[output_index_field, alph_amount_field, address_field];
            let fields = if self.inner.output_index_as_field() {
                all_fields
            } else {
                &all_fields[1..]
            };
            return self.inner.review_fields(fields, review_message);
        }

        let TokenIndexes {
            has_token_metadata,
            token_id,
            token_amount,
        } = token.unwrap();
        let token_id = self.get_str_from_range(token_id)?;
        let token_amount = self.get_str_from_range(token_amount)?;
        let amount_name = if has_token_metadata {
            "Token Amount"
        } else {
            "Raw Token Amount"
        };
        let token_id_field = Field {
            name: "Token ID",
            value: token_id,
        };
        let token_amount_field = Field {
            name: amount_name,
            value: token_amount,
        };
        let all_fields = &[
            output_index_field,
            token_id_field,
            token_amount_field,
            alph_amount_field,
            address_field,
        ];
        let fields: &[Field] = if self.inner.output_index_as_field() {
            all_fields
        } else {
            &all_fields[1..]
        };
        self.inner.review_fields(fields, review_message)
    }

    // Review the transaction details
    pub fn review_tx_details(
        &mut self,
        unsigned_tx: &UnsignedTx,
        device_address: &Address,
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
            }
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
                    if outputs.current_index == 0 {
                        self.inner.start_review()?;
                    }
                    let result =
                        self.review_output(current_output, device_address, temp_data.read_all());
                    self.reset_buffer(self.token_metadata_length);
                    result
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    // Review the rest transaction details and approve it
    pub fn approve_tx(&mut self) -> Result<(), ErrorCode> {
        assert!(self.tx_fee.is_some());
        let mut amount_output = [0u8; 33];
        let amount_str = self
            .tx_fee
            .as_ref()
            .unwrap()
            .to_alph(&mut amount_output)
            .unwrap();
        let value = bytes_to_string(amount_str)?;
        let fee_field = Field {
            name: "Fees",
            value,
        };
        if self.next_output_index == FIRST_OUTPUT_INDEX {
            return self.inner.review_self_transfer(fee_field);
        }

        let fields = &[fee_field];
        self.inner.finish_review(fields)
    }

    pub fn check_blind_signing(&mut self) -> Result<(), ErrorCode> {
        self.inner.check_blind_signing()
    }

    #[cfg(any(target_os = "stax", target_os = "flex"))]
    #[inline]
    pub fn display_settings(&self) -> bool {
        self.inner.display_settings
    }

    #[cfg(any(target_os = "stax", target_os = "flex"))]
    #[inline]
    pub fn reset_display_settings(&mut self) {
        self.inner.reset_display_settings()
    }
}

// Output indexes for review
// The indexes are used to get the values from the buffer
// The values are then used to display the transaction details
// The transaction details are then reviewed by the user
pub struct OutputIndexes {
    pub review_message: (usize, usize),
    pub alph_amount: (usize, usize),
    pub address: (usize, usize),
    pub token: Option<TokenIndexes>,
}

// Token indexes for review
// The indexes are used to get the values from the buffer
// The values are then used to display the token details
// The token details are then reviewed by the user
pub struct TokenIndexes {
    pub has_token_metadata: bool,
    pub token_id: (usize, usize),
    pub token_amount: (usize, usize),
}

#[inline]
fn get_token_symbol_bytes(bytes: &[u8]) -> &[u8] {
    let mut index = 0;
    while index < bytes.len() && bytes[index] != 0 {
        index += 1;
    }
    &bytes[..index]
}
