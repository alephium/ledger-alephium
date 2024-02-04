use super::*;
use crate::buffer::{Buffer, Writable};
use crate::decode::*;

#[cfg_attr(test, derive(Debug))]
pub enum UnsignedTx {
    Version(Byte),
    NetworkId(Byte),
    ScriptOpt(PartialDecoder<Option<Script>>),
    GasAmount(I32),
    GasPrice(U256),
    Inputs(AVector<TxInput>),
    FixedOutputs(AVector<AssetOutput>),
}

impl Reset for UnsignedTx {
    fn reset(&mut self) {
        *self = Self::Version(Byte(0));
    }
}

impl UnsignedTx {
    pub fn is_complete(&self) -> bool {
        match self {
            Self::FixedOutputs(outputs) if outputs.is_complete() => true,
            _ => false,
        }
    }

    #[inline]
    pub fn next_step(&mut self) {
        match self {
            Self::Version(_) => *self = Self::NetworkId(Byte::default()),
            Self::NetworkId(_) => *self = Self::ScriptOpt(PartialDecoder::default()),
            Self::ScriptOpt(_) => *self = Self::GasAmount(I32::default()),
            Self::GasAmount(_) => *self = Self::GasPrice(U256::default()),
            Self::GasPrice(_) => *self = Self::Inputs(AVector::default()),
            Self::Inputs(inputs) => {
                if inputs.is_complete() {
                    *self = Self::FixedOutputs(AVector::default())
                }
            }
            Self::FixedOutputs(_) => (),
        }
    }
}

impl Default for UnsignedTx {
    fn default() -> Self {
        Self::Version(Byte(0))
    }
}

impl RawDecoder for UnsignedTx {
    fn step_size(&self) -> u16 {
        match self {
            Self::Inputs(inputs) => inputs.step_size(),
            Self::FixedOutputs(outputs) => outputs.step_size(),
            _ => 1,
        }
    }

    fn decode<'a, W: Writable>(
        &mut self,
        buffer: &mut Buffer<'a, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        match self {
            Self::Version(byte) => byte.decode(buffer, stage),
            Self::NetworkId(byte) => byte.decode(buffer, stage),
            Self::ScriptOpt(script) => script.decode_children(buffer, stage),
            Self::GasAmount(amount) => amount.decode(buffer, stage),
            Self::GasPrice(amount) => amount.decode(buffer, stage),
            Self::Inputs(inputs) => inputs.decode(buffer, stage),
            Self::FixedOutputs(outputs) => outputs.decode(buffer, stage),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    extern crate std;

    use super::*;
    use crate::buffer::Buffer;
    use crate::decode::new_decoder;
    use crate::types::i32::tests::random_usize;
    use crate::types::u256::tests::hex_to_bytes;
    use crate::types::{AVector, Hash, Hint, LockupScript, PublicKey, UnlockScript, I32, U256};
    use crate::TempData;
    use blake2::{Blake2b, Digest};
    use core::cmp::min;
    use digest::consts::U32;
    use num_bigint::BigUint;
    use std::vec::Vec;

    type Blake2b256 = Blake2b<U32>;

    fn decode<'a, W: Writable>(
        buffer: &mut Buffer<'a, W>,
        decoder: &mut PartialDecoder<UnsignedTx>,
        hasher: &mut Blake2b256,
    ) -> DecodeResult<bool> {
        let from_index = buffer.get_index();
        let result = decoder.try_decode_one_step(buffer);
        let to_index = buffer.get_index();
        hasher.update(buffer.get_range(from_index, to_index));
        result
    }

    fn p2pkh_lockup_script(public_key_hash_hex: &str) -> LockupScript {
        let bytes = hex_to_bytes(public_key_hash_hex).unwrap();
        let hash = Hash::from_bytes(bytes.as_slice().try_into().unwrap());
        LockupScript::P2PKH(hash)
    }

    fn p2pkh_unlock_script(public_key_hex: &str) -> UnlockScript {
        let public_key = PublicKey::from_bytes(
            hex_to_bytes(public_key_hex)
                .unwrap()
                .as_slice()
                .try_into()
                .unwrap(),
        );
        UnlockScript::P2PKH(public_key)
    }

    fn input(hint: i32, key: &str, unlock_script: UnlockScript) -> TxInput {
        let hint_bytes = i32::to_be_bytes(hint);
        let hint = Hint::from_bytes(hint_bytes);
        let key_bytes = hex_to_bytes(key).unwrap();
        let key = Hash::from_bytes(key_bytes.as_slice().try_into().unwrap());
        TxInput {
            hint,
            key,
            unlock_script,
        }
    }

    fn p2pkh_input(hint: i32, key: &str, public_key_hex: &str) -> TxInput {
        let unlock_script = p2pkh_unlock_script(public_key_hex);
        input(hint, key, unlock_script)
    }

    fn u256_from_str(amount_str: &str) -> U256 {
        let number = BigUint::parse_bytes(amount_str.as_bytes(), 10).unwrap();
        let mut bytes = number.to_bytes_be();
        assert!(bytes.len() > 4);
        let header: u8 = ((bytes.len() - 4) as u8) | 0xc0;
        bytes.insert(0, header);
        U256::from_encoded_bytes(&bytes)
    }

    fn output(amount_str: &str, lockup_script: LockupScript) -> AssetOutput {
        let amount = u256_from_str(amount_str);
        AssetOutput {
            amount,
            lockup_script,
            lock_time: TimeStamp(0),
            tokens: AVector::default(),
            additional_data: ByteString::empty(),
        }
    }

    fn token(token_id_hex: &str, amount_str: &str) -> Token {
        let token_id_bytes = hex_to_bytes(token_id_hex).unwrap();
        let token_id = Hash::from_bytes(token_id_bytes.as_slice().try_into().unwrap());
        let amount = u256_from_str(amount_str);
        Token {
            id: token_id,
            amount,
        }
    }

    fn p2pkh_output_with_token(
        amount_str: &str,
        public_key_hash_hex: &str,
        token_id_hex: &str,
        token_amount: &str,
    ) -> AssetOutput {
        let output = p2pkh_output(amount_str, public_key_hash_hex);
        let token = token(token_id_hex, token_amount);
        AssetOutput {
            tokens: AVector::from_item(token),
            ..output
        }
    }

    fn p2pkh_output(amount_str: &str, public_key_hash_hex: &str) -> AssetOutput {
        output(amount_str, p2pkh_lockup_script(public_key_hash_hex))
    }

    fn decode_and_check_tx(
        tx_id_hex: &str,
        encoded_tx: Vec<u8>,
        gas_amount: I32,
        gas_price: U256,
        is_script_tx: bool,
        all_inputs: &[TxInput],
        all_outputs: &[AssetOutput],
    ) {
        let check = |tx: &UnsignedTx| match tx {
            UnsignedTx::Version(byte) => assert_eq!(byte.0, 0),
            UnsignedTx::NetworkId(byte) => assert_eq!(byte.0, 0),
            UnsignedTx::ScriptOpt(script) => {
                if is_script_tx {
                    assert!(script.inner.is_some())
                } else {
                    assert!(script.inner.is_none())
                }
            }
            UnsignedTx::GasAmount(amount) => assert_eq!(amount, &gas_amount),
            UnsignedTx::GasPrice(amount) => assert_eq!(amount, &gas_price),
            UnsignedTx::Inputs(inputs) => {
                let current_input = inputs.get_current_item();
                if current_input.is_some() {
                    assert_eq!(
                        current_input.unwrap(),
                        &all_inputs[inputs.current_index as usize]
                    );
                }
            }
            UnsignedTx::FixedOutputs(outputs) => {
                let current_output = outputs.get_current_item();
                if current_output.is_some() {
                    assert_eq!(
                        current_output.unwrap(),
                        &all_outputs[outputs.current_index as usize]
                    );
                }
            }
        };

        let frame_sizes = [1, u8::MAX as usize];
        for frame_size in frame_sizes {
            let mut length: usize = 0;
            let mut decoder = new_decoder::<UnsignedTx>();
            let mut hasher = Blake2b256::new();
            let mut temp_data = TempData::new();

            while length < encoded_tx.len() {
                let remain = encoded_tx.len() - length;
                let size = min(random_usize(0, frame_size), remain);
                let mut buffer =
                    Buffer::new(&encoded_tx[length..(length + size)], &mut temp_data).unwrap();
                length += size;

                let mut continue_decode = true;
                while continue_decode {
                    let result = decode(&mut buffer, &mut decoder, &mut hasher).unwrap();
                    if result {
                        check(&decoder.inner);
                        decoder.inner.next_step();
                        decoder.reset_stage();
                    } else {
                        continue_decode = false;
                    }
                    if decoder.inner.is_complete() {
                        continue_decode = false;
                    }
                }
            }

            let tx_id = hasher.finalize().to_vec();
            assert_eq!(tx_id, hex_to_bytes(tx_id_hex).unwrap());
        }
    }

    #[test]
    fn test_decode_transfer_alph_tx() {
        let tx_id_hex = "c53f150bceb13c6ca1c13fee897e688c0ef86c73ad8113edf444b7b15ecf438b";
        let encoded_tx = hex_to_bytes("0000008000de1cc1174876e80006cb6501716c7f09df51c6e9d2412210f756dd13b12914ace98be11a506468bbc09b4457f30002622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235ecb650171c5b1c7ec8f38a446b5824ab3b4785eb59813be6309caccf09e81badba48875550002622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235ecb650171c8fc4448bd13db645484b628da13e8e95d0c1c7f63d93e2d2098dac7c902dec30002622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235ecb6501715faa376ca823d5a3bf265ff932e3ddc695b87d7d577e6c77277a96756d42cd430002622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235ecb6501716fc17b71c1a8be6f822b74d991675535cb55af5835d7f2ed146f769323c3e9450002622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235ecb650171950bf46c8d7fe6ca54a2cffdbc29f60c9b666fb42cb1c09a17d2ff555e3e893e0002622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235e07c4145b402ea4c0cb000038f63ae3338e738b288103aa3d4cab822a8bfaf19ace50798bd4c8439f06c55700000000000000000000c40eb17f1ebec364c000f933eafd1dd5d5ac00d6eac5dd0f54e527e72aa8d82f81701ae6b8e481d9708500000000000000000000c40ed336ec389dffc0002f53372b89cbe04a208643ccf098561ea545fdb121359df48378e828dbb3ef1100000000000000000000c48b127aec9cc8068000102bdf758a5fb7c1f049e75c7d297f1aa7d84d74eeaf9cee2b388d1fc94ec48000000000000000000000c40de259e640f7c040007720aecb72dfa949eefe173bdff8223346384b564389533bd267ecdfe8dcdadc00000000000000000000c40e4568375f83f5c000df1562ff1670a6d955d1f7c27d6319289b1fc358bf357adf97d5f097a6895f0a00000000000000000000c44ec157b933227c80009b85f066b1b2821339bf73e9e00bbe660b0cfb97158ceedff3260e1e4368961d00000000000000000000").unwrap();

        let gas_amount = I32::from(56860);
        let gas_price = U256::from_encoded_bytes(&[0xc1, 0x17, 0x48, 0x76, 0xe8, 0x00]);
        let all_inputs = [
            p2pkh_input(
                -882572943,
                "6c7f09df51c6e9d2412210f756dd13b12914ace98be11a506468bbc09b4457f3",
                "02622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235e",
            ),
            p2pkh_input(
                -882572943,
                "c5b1c7ec8f38a446b5824ab3b4785eb59813be6309caccf09e81badba4887555",
                "02622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235e",
            ),
            p2pkh_input(
                -882572943,
                "c8fc4448bd13db645484b628da13e8e95d0c1c7f63d93e2d2098dac7c902dec3",
                "02622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235e",
            ),
            p2pkh_input(
                -882572943,
                "5faa376ca823d5a3bf265ff932e3ddc695b87d7d577e6c77277a96756d42cd43",
                "02622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235e",
            ),
            p2pkh_input(
                -882572943,
                "6fc17b71c1a8be6f822b74d991675535cb55af5835d7f2ed146f769323c3e945",
                "02622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235e",
            ),
            p2pkh_input(
                -882572943,
                "950bf46c8d7fe6ca54a2cffdbc29f60c9b666fb42cb1c09a17d2ff555e3e893e",
                "02622da4723abe3e57e6926b69a049635dad0f9059a89ca222d83f0b2da256235e",
            ),
        ];

        let all_outputs = [
            p2pkh_output(
                "1466836672716000000",
                "38f63ae3338e738b288103aa3d4cab822a8bfaf19ace50798bd4c8439f06c557",
            ),
            p2pkh_output(
                "1058767157435000000",
                "f933eafd1dd5d5ac00d6eac5dd0f54e527e72aa8d82f81701ae6b8e481d97085",
            ),
            p2pkh_output(
                "1068257924807000000",
                "2f53372b89cbe04a208643ccf098561ea545fdb121359df48378e828dbb3ef11",
            ),
            p2pkh_output(
                "10021207277514000000",
                "102bdf758a5fb7c1f049e75c7d297f1aa7d84d74eeaf9cee2b388d1fc94ec480",
            ),
            p2pkh_output(
                "1000460912697000000",
                "7720aecb72dfa949eefe173bdff8223346384b564389533bd267ecdfe8dcdadc",
            ),
            p2pkh_output(
                "1028342676959000000",
                "df1562ff1670a6d955d1f7c27d6319289b1fc358bf357adf97d5f097a6895f0a",
            ),
            p2pkh_output(
                "5674913458402000000",
                "9b85f066b1b2821339bf73e9e00bbe660b0cfb97158ceedff3260e1e4368961d",
            ),
        ];

        decode_and_check_tx(
            tx_id_hex,
            encoded_tx,
            gas_amount,
            gas_price,
            false,
            &all_inputs,
            &all_outputs,
        );
    }

    #[test]
    fn test_decode_transfer_token_tx() {
        let tx_id_hex = "668827ae5719d8acb7efa4e8684cd3968738736833369ad56482b7ccb6bad5c7";
        let encoded_tx = hex_to_bytes("000000800079ccc1174876e80003f6179435b26eb070309593a0aa5eef3f1ae3f7337a0dba1e7d94f3d8c4adc2743636057c0002e835a6e954a0a0b0e540f4451186e5a1f99baf93a111d304866945a768c39d5cf61794350817b6c1ea8fae4a48fb6868d8f47147ef8bd62a92589a876419352dfc5103610002e835a6e954a0a0b0e540f4451186e5a1f99baf93a111d304866945a768c39d5cf61794353cfed394414a0238ab8be798b88140c4f9255f094f30614f184afa0ba5984ba00002e835a6e954a0a0b0e540f4451186e5a1f99baf93a111d304866945a768c39d5c04c3038d7ea4c6800000bee85f379545a2ed9f6cceb331288842f378cf0f04012ad4ac8824aae7d6f80a0000000000000000011a281053ba8601a658368594da034c2e99a0fb951b86498d05e76aedfe666800c3038d7ea4c6800000c40c79e3bca513800000bee85f379545a2ed9f6cceb331288842f378cf0f04012ad4ac8824aae7d6f80a00000000000000000000c3038d7ea4c68000004e796b6f3b889eb8959c285ea4ef8dea6d7aad4c444e2f83f3403fdfde5d2eb60000000000000000011a281053ba8601a658368594da034c2e99a0fb951b86498d05e76aedfe666800c302dd4700d857d600c438a38658095af000004e796b6f3b889eb8959c285ea4ef8dea6d7aad4c444e2f83f3403fdfde5d2eb600000000000000000000").unwrap();

        let gas_amount = I32::from(31180);
        let gas_price = U256::from_encoded_bytes(&[0xc1, 0x17, 0x48, 0x76, 0xe8, 0x00]);
        let all_inputs = [
            p2pkh_input(
                -166226891,
                "b26eb070309593a0aa5eef3f1ae3f7337a0dba1e7d94f3d8c4adc2743636057c",
                "02e835a6e954a0a0b0e540f4451186e5a1f99baf93a111d304866945a768c39d5c",
            ),
            p2pkh_input(
                -166226891,
                "0817b6c1ea8fae4a48fb6868d8f47147ef8bd62a92589a876419352dfc510361",
                "02e835a6e954a0a0b0e540f4451186e5a1f99baf93a111d304866945a768c39d5c",
            ),
            p2pkh_input(
                -166226891,
                "3cfed394414a0238ab8be798b88140c4f9255f094f30614f184afa0ba5984ba0",
                "02e835a6e954a0a0b0e540f4451186e5a1f99baf93a111d304866945a768c39d5c",
            ),
        ];

        let all_outputs = [
            p2pkh_output_with_token(
                "1000000000000000",
                "bee85f379545a2ed9f6cceb331288842f378cf0f04012ad4ac8824aae7d6f80a",
                "1a281053ba8601a658368594da034c2e99a0fb951b86498d05e76aedfe666800",
                "1000000000000000",
            ),
            p2pkh_output(
                "899000000000000000",
                "bee85f379545a2ed9f6cceb331288842f378cf0f04012ad4ac8824aae7d6f80a",
            ),
            p2pkh_output_with_token(
                "1000000000000000",
                "4e796b6f3b889eb8959c285ea4ef8dea6d7aad4c444e2f83f3403fdfde5d2eb6",
                "1a281053ba8601a658368594da034c2e99a0fb951b86498d05e76aedfe666800",
                "806246980016086",
            ),
            p2pkh_output(
                "4081253400000000000",
                "4e796b6f3b889eb8959c285ea4ef8dea6d7aad4c444e2f83f3403fdfde5d2eb6",
            ),
        ];

        decode_and_check_tx(
            tx_id_hex,
            encoded_tx,
            gas_amount,
            gas_price,
            false,
            &all_inputs,
            &all_outputs,
        );
    }

    #[test]
    fn test_decode_script_tx() {
        let tx_id_hex = "b4d93868e9b20c2757067334799ea815614fcec306eb254832dbbbd58eb8d42a";
        let encoded_tx = hex_to_bytes("0000010101030001000b1440205bf2f559ae714dab83ff36bed4d9e634dfda3ca9ed755d60f00be89e2a20bd001700b4160013c5056bc75e2d63100000a313c5056bc75e2d631000000d0c1440205bf2f559ae714dab83ff36bed4d9e634dfda3ca9ed755d60f00be89e2a20bd00010e8000bffcc1174876e80002e412bbf9030c20b11b0d1755c76eca9aee0144286933d46bfadbdd0b59976ae73e67523000037fda053ebb06b77a9b03ba029f826ec3e1337e47462743bc0b5035ec0d033615e412bbf93f98f4e88567ca1b978d5a59b126fa8afd7432231c8217e2684e99d3d686826e00037fda053ebb06b77a9b03ba029f826ec3e1337e47462743bc0b5035ec0d03361502c3038d7ea4c68000005bb4d7a6644d4981818916b1d480335290ec9c38beacb827fe92dde7cab5698d0000000000000000015bf2f559ae714dab83ff36bed4d9e634dfda3ca9ed755d60f00be89e2a20bd00c50d49f0894c3e0c685800c530759dc0cd56ff0000005bb4d7a6644d4981818916b1d480335290ec9c38beacb827fe92dde7cab5698d00000000000000000000").unwrap();

        let gas_amount = I32::from(49148);
        let gas_price = U256::from_encoded_bytes(&[0xc1, 0x17, 0x48, 0x76, 0xe8, 0x00]);
        let all_inputs = [
            p2pkh_input(
                -468534279,
                "030c20b11b0d1755c76eca9aee0144286933d46bfadbdd0b59976ae73e675230",
                "037fda053ebb06b77a9b03ba029f826ec3e1337e47462743bc0b5035ec0d033615",
            ),
            p2pkh_input(
                -468534279,
                "3f98f4e88567ca1b978d5a59b126fa8afd7432231c8217e2684e99d3d686826e",
                "037fda053ebb06b77a9b03ba029f826ec3e1337e47462743bc0b5035ec0d033615",
            ),
        ];

        let all_outputs = [
            p2pkh_output_with_token(
                "1000000000000000",
                "5bb4d7a6644d4981818916b1d480335290ec9c38beacb827fe92dde7cab5698d",
                "5bf2f559ae714dab83ff36bed4d9e634dfda3ca9ed755d60f00be89e2a20bd00",
                "245135582277954988120",
            ),
            p2pkh_output(
                "893918857600000000000",
                "5bb4d7a6644d4981818916b1d480335290ec9c38beacb827fe92dde7cab5698d",
            ),
        ];

        decode_and_check_tx(
            tx_id_hex,
            encoded_tx,
            gas_amount,
            gas_price,
            true,
            &all_inputs,
            &all_outputs,
        );
    }

    #[test]
    fn test_decode_coinbase_tx() {
        let tx_id_hex = "a720a161efca30b9378da93facf1fa5fc9340ffb17e1f859f1100fa1e0b61038";
        let encoded_tx = hex_to_bytes("00000080004e20bb9aca000001c4212afc56552f000000edae9a1e22e324a9997a1dc522ee4b3a99bb38e3a35ee4ebd147396a4a9893160000018d1e54526c000a00000000018d1c8a8eec").unwrap();

        let gas_amount = I32::from(20000);
        let gas_price = U256::from_encoded_bytes(&[0xbb, 0x9a, 0xca, 0x00]);
        let all_inputs = [];

        let output = p2pkh_output(
            "2390000000000000000",
            "edae9a1e22e324a9997a1dc522ee4b3a99bb38e3a35ee4ebd147396a4a989316",
        );
        let all_outputs = [AssetOutput {
            lock_time: TimeStamp(1705610859116),
            additional_data: ByteString {
                length: I32::from(10),
                current_index: 10,
            },
            ..output
        }];

        decode_and_check_tx(
            tx_id_hex,
            encoded_tx,
            gas_amount,
            gas_price,
            false,
            &all_inputs,
            &all_outputs,
        );
    }
}
