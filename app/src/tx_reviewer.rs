use crate::{blake2b_hasher::Blake2bHasher, error_code::ErrorCode, nvm_buffer::NVM};
use core::str::from_utf8;
use ledger_device_sdk::{
    ui::{
        bitmaps::{CHECKMARK, CROSS, EYE},
        gadgets::{Field, MultiFieldReview},
    },
    Pic,
};
use utils::{
    base58::{base58_encode_inputs, ALPHABET},
    types::{AssetOutput, Byte32, LockupScript, TxInput, UnlockScript, I32, U256},
};

const SIZE: usize = 2048;

#[link_section = ".rodata.N_"]
static mut DATA: Pic<NVM<SIZE>> = Pic::new(NVM::zeroed());

pub struct TxReviewer {
    data: &'static mut Pic<NVM<SIZE>>,
    index: usize,
}

impl TxReviewer {
    pub fn new() -> Self {
        unsafe {
            Self {
                data: &mut DATA,
                index: 0,
            }
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.index = 0;
    }

    fn write_from(&mut self, from_index: usize, bytes: &[u8]) -> Result<(), ErrorCode> {
        let data = self.data.get_mut();
        if data.write(from_index, bytes) {
            Ok(())
        } else {
            Err(ErrorCode::Overflow)
        }
    }

    fn write_alph_amount(&mut self, u256: &U256) -> Result<usize, ErrorCode> {
        let mut amount_output = [0u8; 33];
        let amount_str = u256.to_alph(&mut amount_output).unwrap();
        self.write_str(amount_str)
    }

    fn write_token_amount(&mut self, u256: &U256) -> Result<usize, ErrorCode> {
        // TODO: improve this
        let mut amount_output = [0u8; 39]; // u128 max
        let amount_str = u256.to_str(&mut amount_output).unwrap();
        self.write_str(amount_str)
    }

    fn write_str(&mut self, str: &[u8]) -> Result<usize, ErrorCode> {
        let size = get_memory_size(str.len());
        self.write_from(self.index, str)?;
        let to_index = self.index + str.len();
        self.index += size;
        Ok(to_index)
    }

    fn write_token_id(&mut self, token_id: &Byte32) -> Result<usize, ErrorCode> {
        let hex_str: [u8; 64] = utils::to_hex(&token_id.0).unwrap();
        self.write_str(&hex_str)
    }

    fn read_from_range(&self, from_index: usize, to_index: usize) -> &[u8] {
        &self.data.get_ref().0[from_index..to_index]
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
            let stored = self.read_from_range(from_index, from_index + 64);
            for index in 0..64 {
                new_carry += (stored[index] as usize) << 8;
                bytes[index] = (new_carry % 58) as u8;
                new_carry /= 58;
            }
            self.write_from(from_index, &bytes)?;
            bytes = [0; 64];
            from_index += 64;
        }
        Ok(new_carry)
    }

    fn finalize_multi_sig(&mut self, from: usize, to: usize) -> Result<(), ErrorCode> {
        assert!(to - from > 64);
        let mut temp0 = [0u8; 64];
        let mut temp1 = [0u8; 64];
        let mut begin = from;
        let mut end = to;
        while begin < end {
            if (end - begin) <= 64 {
                let stored = self.read_from_range(begin, end);
                let length = end - begin;
                for i in 0..length {
                    temp0[length - i - 1] = ALPHABET[stored[i] as usize];
                }
                self.write_from(begin, &temp0[..length])?;
                return Ok(());
            }

            let left = self.read_from_range(begin, begin + 64);
            let right = self.read_from_range(end - 64, end);
            for i in 0..64 {
                let index = 64 - i - 1;
                temp0[index] = ALPHABET[left[i] as usize];
                temp1[index] = ALPHABET[right[i] as usize];
            }
            self.write_from(begin, &temp1)?;
            self.write_from(end - 64, &temp0)?;
            end -= 64;
            begin += 64;
        }
        Ok(())
    }

    // This function only for multi-sig address, which has no leading zeros
    pub fn write_multi_sig(&mut self, input: &[u8]) -> Result<usize, ErrorCode> {
        let from_index = self.index;
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
                    self.write_from(from_index + output_length, &output)?;
                    output = [0u8; 64];
                    output_length += 64;
                }
                output[output_index - output_length] = (carry % 58) as u8;
                output_index += 1;
                carry /= 58;
            }
        }

        self.write_from(
            from_index + output_length,
            &output[..(output_index - output_length)],
        )?;
        self.finalize_multi_sig(from_index, from_index + output_index)?;

        let result = from_index + output_index;
        self.index += get_memory_size(output_index);
        Ok(result)
    }

    fn write_index_with_prefix(&mut self, index: usize, prefix: &[u8]) -> Result<usize, ErrorCode> {
        let mut output = [0u8; 20];
        assert!(prefix.len() + 3 <= 20);
        let num = I32::unsafe_from(index);
        let mut num_output: [u8; 3] = [0u8; 3];
        let num_str_bytes = num.to_str(&mut num_output);
        if num_str_bytes.is_none() {
            return Err(ErrorCode::Overflow);
        }
        output[..prefix.len()].copy_from_slice(prefix);
        let num_str = num_str_bytes.unwrap();
        let total_size = prefix.len() + num_str.len();
        output[prefix.len()..total_size].copy_from_slice(num_str);
        self.write_str(&output[..total_size])
    }

    pub fn write_address(&mut self, prefix: u8, hash: &[u8; 32]) -> Result<usize, ErrorCode> {
        let mut output = [0u8; 46];
        let str_bytes = base58_encode_inputs(&[&[prefix], &hash[..]], &mut output);
        if str_bytes.is_none() {
            return Err(ErrorCode::Overflow);
        }
        self.write_str(&str_bytes.unwrap())
    }

    fn prepare_input(
        &mut self,
        input: &TxInput,
        current_index: usize,
        temp_data: &[u8],
    ) -> Result<InputIndexes, ErrorCode> {
        let review_message_from_index = self.index;
        let review_message_to_index =
            self.write_index_with_prefix(current_index, b"Review Input #")?;

        let address_from_index = self.index;
        let address_to_index = match &input.unlock_script {
            UnlockScript::P2PKH(public_key) => {
                let public_key_hash = Blake2bHasher::hash(&public_key.0)?;
                self.write_address(0u8, &public_key_hash)?
            }
            UnlockScript::P2MPKH(_) => {
                // TODO: we can't display this address, check if blind signing is enabled
                self.write_str(b"multi-sig address")?
            }
            UnlockScript::P2SH(_) => {
                let script_hash = Blake2bHasher::hash(temp_data)?;
                self.write_address(2u8, &script_hash)?
            }
            UnlockScript::SameAsPrevious => self.write_str(b"same as previous")?,
            _ => panic!(), // dead branch
        };

        Ok(InputIndexes {
            review_message: (review_message_from_index, review_message_to_index),
            address: (address_from_index, address_to_index),
        })
    }

    fn prepare_output(
        &mut self,
        output: &AssetOutput,
        current_index: usize,
        temp_data: &[u8],
    ) -> Result<OutputIndexes, ErrorCode> {
        let review_message_from_index = self.index;
        let review_message_to_index =
            self.write_index_with_prefix(current_index, b"Review Output #")?;

        let alph_amount_from_index = self.index;
        let alph_amount_to_index = self.write_alph_amount(&output.amount)?;

        let address_from_index = self.index;
        let address_to_index = match &output.lockup_script {
            LockupScript::P2PKH(hash) | LockupScript::P2SH(hash) => {
                self.write_address(output.lockup_script.get_type(), &hash.0)?
            }
            LockupScript::P2MPKH(_) => self.write_multi_sig(temp_data)?,
            _ => panic!(), // dead branch
        };

        let output_indexes = OutputIndexes {
            review_message: (review_message_from_index, review_message_to_index),
            alph_amount: (alph_amount_from_index, alph_amount_to_index),
            address: (address_from_index, address_to_index),
            token: None,
        };
        if output.tokens.is_empty() {
            return Ok(output_indexes);
        }

        // Asset output has at most one token
        let token = output.tokens.get_current_item().unwrap();
        let token_id_from_index = self.index;
        let token_id_to_address = self.write_token_id(&token.id)?;

        let token_amount_from_index = self.index;
        let token_amount_to_index = self.write_token_amount(&token.amount)?;

        Ok(OutputIndexes {
            token: Some(TokenIndexes {
                token_id: (token_id_from_index, token_id_to_address),
                token_amount: (token_amount_from_index, token_amount_to_index),
            }),
            ..output_indexes
        })
    }

    fn get_str_from_range(&self, range: (usize, usize)) -> Result<&str, ErrorCode> {
        let bytes = self.read_from_range(range.0, range.1);
        bytes_to_string(bytes)
    }

    pub fn review_network(id: u8) -> Result<(), ErrorCode> {
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

    pub fn review_gas_amount(gas_amount: &I32) -> Result<(), ErrorCode> {
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

    pub fn review_gas_price(&mut self, gas_price: &U256) -> Result<(), ErrorCode> {
        let from_index = self.index;
        let to_index = self.write_alph_amount(gas_price)?;
        let value = self.get_str_from_range((from_index, to_index))?;
        let fields = [Field {
            name: "GasPrice",
            value,
        }];
        review(&fields, "Review Gas Price")
    }

    pub fn review_input(
        &mut self,
        input: &TxInput,
        current_index: usize,
        temp_data: &[u8],
    ) -> Result<(), ErrorCode> {
        let InputIndexes {
            review_message,
            address,
        } = self.prepare_input(input, current_index, temp_data)?;
        let review_message = self.get_str_from_range(review_message)?;
        let address = self.get_str_from_range(address)?;
        let fields = [Field {
            name: "Address",
            value: address,
        }];
        review(&fields, review_message)
    }

    pub fn review_output(
        &mut self,
        output: &AssetOutput,
        current_index: usize,
        temp_data: &[u8],
    ) -> Result<(), ErrorCode> {
        let OutputIndexes {
            review_message,
            alph_amount,
            address,
            token,
        } = self.prepare_output(output, current_index, temp_data)?;
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
            return review(&fields, review_message);
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
                name: "TokenId",
                value: token_id,
            },
            Field {
                name: "TokenAmount",
                value: token_amount,
            },
        ];
        review(&fields, review_message)
    }

    pub fn review_tx_id(tx_id: &[u8; 32]) -> Result<(), ErrorCode> {
        let hex: [u8; 64] = utils::to_hex(&tx_id[..]).unwrap();
        let hex_str = bytes_to_string(&hex)?;
        let fields = [Field {
            name: "TxId",
            value: hex_str,
        }];
        review(&fields, "Review Tx Id")
    }
}

pub struct InputIndexes {
    pub review_message: (usize, usize),
    pub address: (usize, usize),
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

// https://developers.ledger.com/docs/device-app/develop/sdk/memory/persistent-storage#flash-memory-endurance
fn get_memory_size(num: usize) -> usize {
    let remainder = num % 64;
    if remainder == 0 {
        num
    } else {
        num + (64 - remainder)
    }
}

#[inline]
fn bytes_to_string(bytes: &[u8]) -> Result<&str, ErrorCode> {
    from_utf8(bytes).map_err(|_| ErrorCode::InternalError)
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
