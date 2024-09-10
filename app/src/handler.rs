use ledger_device_sdk::io::{self, ApduHeader};
use utils::deserialize_path;

use crate::{
    debug::print::{println, println_slice},
    error_code::ErrorCode,
    public_key::{derive_pub_key, Address},
    sign_tx_context::SignTxContext,
    ui::{bytes_to_string, review_address, sign_hash_ui, tx_reviewer::TxReviewer},
};

const MAX_TOKEN_SIZE: u8 = 5;
const PATH_LENGTH: usize = 20;
const HASH_LENGTH: usize = 32;
const PATH_HEX_LENGTH: usize = PATH_LENGTH * 2;
const CALL_CONTRACT_FLAG: u8 = 0x01;
const SCRIPT_OFFSET: usize = 3; // the encoded script offset in the tx
pub const TOKEN_METADATA_SIZE: usize = 46;

#[repr(u8)]
pub enum Ins {
    GetVersion,
    GetPubKey,
    SignHash,
    SignTx,
}

impl TryFrom<io::ApduHeader> for Ins {
    type Error = ErrorCode;
    fn try_from(header: io::ApduHeader) -> Result<Self, Self::Error> {
        match header.ins {
            0 => Ok(Ins::GetVersion),
            1 => Ok(Ins::GetPubKey),
            2 => Ok(Ins::SignHash),
            3 => Ok(Ins::SignTx),
            _ => Err(ErrorCode::BadIns),
        }
    }
}

pub fn handle_apdu(
    comm: &mut io::Comm,
    ins: Ins,
    sign_tx_context: &mut SignTxContext,
    tx_reviewer: &mut TxReviewer,
) -> Result<(), io::Reply> {
    if comm.rx == 0 {
        return Err(ErrorCode::BadLen.into());
    }

    let mut path: [u32; 5] = [0; 5];
    let apdu_header = comm.get_apdu_metadata();
    if apdu_header.cla != 0x80 {
        return Err(ErrorCode::BadCla.into());
    }

    // Common instructions
    match ins {
        Ins::GetVersion => {
            let version_major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u8>().unwrap();
            let version_minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u8>().unwrap();
            let version_patch = env!("CARGO_PKG_VERSION_PATCH").parse::<u8>().unwrap();
            comm.append([version_major, version_minor, version_patch].as_slice());
        }
        Ins::GetPubKey => {
            let data = comm.get_data()?;
            // 1 byte flag indicating whether address verification is needed
            if data.len() != PATH_LENGTH + 1 {
                return Err(ErrorCode::BadLen.into());
            }
            let raw_path = &data[..PATH_LENGTH];
            deserialize_path::<io::Reply>(
                raw_path,
                &mut path,
                ErrorCode::HDPathDecodingFailed.into(),
            )?;

            println("raw path");
            println_slice::<PATH_HEX_LENGTH>(raw_path);
            let p1 = apdu_header.p1; // Group number: 0 for all groups
            let p2 = apdu_header.p2; // Target group
            let (pk, hd_index) = derive_pub_key(&mut path, p1, p2)?;

            let need_to_display = data[PATH_LENGTH] != 0;
            if need_to_display {
                let address = Address::from_pub_key(&pk)?;
                let address_str = bytes_to_string(address.get_address_bytes())?;
                review_address(address_str)?;
            }

            comm.append(pk.as_ref());
            comm.append(hd_index.to_be_bytes().as_slice());
        }
        Ins::SignHash => {
            let data = comm.get_data()?;
            if data.len() != PATH_LENGTH + HASH_LENGTH {
                return Err(ErrorCode::BadLen.into());
            }
            // This check can be removed, but we keep it for double checking
            deserialize_path::<io::Reply>(
                &data[..PATH_LENGTH],
                &mut path,
                ErrorCode::HDPathDecodingFailed.into(),
            )?;

            match sign_hash_ui(&path, &data[PATH_LENGTH..]) {
                Ok((signature_buf, length, _)) => comm.append(&signature_buf[..length as usize]),
                Err(code) => return Err(code.into()),
            }
        }
        Ins::SignTx => {
            let data = match comm.get_data() {
                Ok(data) => data,
                Err(code) => {
                    reset(sign_tx_context, tx_reviewer);
                    return Err(code.into());
                }
            };
            match handle_sign_tx(apdu_header, data, sign_tx_context, tx_reviewer) {
                Ok(()) if !sign_tx_context.is_complete() => {
                    return Ok(());
                }
                Ok(()) => {
                    // The transaction is signed when all the data is processed
                    // The signature is returned in the response
                    let sign_result = tx_reviewer
                        .approve_tx()
                        .and_then(|_| sign_tx_context.sign_tx());
                    let result = match sign_result {
                        Ok((signature_buf, length, _)) => {
                            comm.append(&signature_buf[..length as usize]);
                            Ok(())
                        }
                        Err(code) => Err(code.into()),
                    };
                    reset(sign_tx_context, tx_reviewer);
                    return result;
                }
                Err(code) => {
                    reset(sign_tx_context, tx_reviewer);
                    return Err(code.into());
                }
            }
        }
    }
    Ok(())
}

// The transaction is split into multiple APDU commands, consisting of token metadata APDU and tx APDU commands
// We use `p1` and `p2` to distinguish between APDUs:
// * `p1` = 0 and `p2` = 0 indicates the first token metadata APDU frame
// * `p1` = 0 and `p2` = 1 indicates a new token metadata APDU frame
// * `p1` = 0 and `p2` = 2 indicates the remaining token proof APDU frame
// * `p1` = 1 and `p2` = 0 indicates the first tx APDU frame
// * `p1` = 1 and `p2` = 1 indicates subsequent tx APDU frames
fn handle_sign_tx(
    apdu_header: &ApduHeader,
    data: &[u8],
    sign_tx_context: &mut SignTxContext,
    tx_reviewer: &mut TxReviewer,
) -> Result<(), ErrorCode> {
    match (apdu_header.p1, apdu_header.p2) {
        (0, 0) => {
            // the first frame
            if data.is_empty() {
                return Err(ErrorCode::BadLen);
            }
            let token_size = data[0]; // the first byte is the token size
            check_token_size(token_size)?;
            tx_reviewer.init(token_size)?;
            if token_size == 0 {
                return Ok(());
            }
            tx_reviewer.handle_token_metadata(&data[1..])
        }
        (0, 1) => tx_reviewer.handle_token_metadata(data), // token metadata and proof frame
        (0, 2) => tx_reviewer.handle_token_proof(data),    // the following token proof frame
        (1, 0) => {
            // the first unsigned tx frame
            if data.len() < PATH_LENGTH + SCRIPT_OFFSET {
                return Err(ErrorCode::BadLen);
            }
            let tx_data = &data[PATH_LENGTH..];
            let is_tx_execute_script = tx_data[SCRIPT_OFFSET - 1] == CALL_CONTRACT_FLAG;
            if is_tx_execute_script {
                tx_reviewer.check_blind_signing()?;
            }
            tx_reviewer.set_tx_execute_script(is_tx_execute_script);

            sign_tx_context.init(&data[..PATH_LENGTH])?;
            sign_tx_context.handle_tx_data(apdu_header, tx_data, tx_reviewer)
        }
        (1, 1) => sign_tx_context.handle_tx_data(apdu_header, data, tx_reviewer), // the following unsigned tx frame
        _ => Err(ErrorCode::BadP1P2),
    }
}

#[inline]
fn check_token_size(size: u8) -> Result<(), ErrorCode> {
    if size > MAX_TOKEN_SIZE {
        Err(ErrorCode::InvalidTokenSize)
    } else {
        Ok(())
    }
}

#[inline]
fn reset(sign_tx_context: &mut SignTxContext, tx_reviewer: &mut TxReviewer) {
    sign_tx_context.reset();
    tx_reviewer.reset();
}
