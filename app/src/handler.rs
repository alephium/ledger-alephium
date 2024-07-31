use ledger_device_sdk::io::{self, ApduHeader};
use utils::deserialize_path;

use crate::{
    debug::print::{println, println_slice},
    error_code::ErrorCode,
    public_key::derive_pub_key,
    sign_tx_context::{check_blind_signing, SignTxContext},
    ui::{
        review_address, sign_hash_ui,
        tx_reviewer::{TxReviewer, TOKEN_METADATA_SIZE},
    },
};

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

    match ins {
        Ins::GetVersion => {
            let version_major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u8>().unwrap();
            let version_minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u8>().unwrap();
            let version_patch = env!("CARGO_PKG_VERSION_PATCH").parse::<u8>().unwrap();
            comm.append([version_major, version_minor, version_patch].as_slice());
        }
        Ins::GetPubKey => {
            let data = comm.get_data()?;
            if data.len() != 21 {
                return Err(ErrorCode::BadLen.into());
            }
            let raw_path = &data[..20];
            if !deserialize_path(raw_path, &mut path) {
                return Err(ErrorCode::BadLen.into());
            }

            println("raw path");
            println_slice::<40>(raw_path);
            let p1 = apdu_header.p1;
            let p2 = apdu_header.p2;
            let (pk, hd_index) = derive_pub_key(&mut path, p1, p2)?;

            let need_to_display = data[20] != 0;
            if need_to_display {
                review_address(&pk)?;
            }

            comm.append(pk.as_ref());
            comm.append(hd_index.to_be_bytes().as_slice());
        }
        Ins::SignHash => {
            let data = comm.get_data()?;
            if data.len() != 4 * 5 + 32 {
                return Err(ErrorCode::BadLen.into());
            }
            // This check can be removed, but we keep it for double checking
            if !deserialize_path(&data[..20], &mut path) {
                return Err(ErrorCode::BadLen.into());
            }

            match sign_hash_ui(&path, &data[20..]) {
                Ok((signature_buf, length, _)) => comm.append(&signature_buf[..length as usize]),
                Err(code) => return Err(code.into()),
            }
        }
        Ins::SignTx => {
            let data = comm.get_data()?;
            match handle_sign_tx(apdu_header, data, sign_tx_context, tx_reviewer) {
                Ok(()) if !sign_tx_context.is_complete() => {
                    return Ok(());
                }
                Ok(()) => {
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
                    return result;
                }
                Err(code) => {
                    return Err(code.into());
                }
            }
        }
    }
    Ok(())
}

const MAX_TOKEN_SIZE: u8 = 5;

fn handle_sign_tx(
    apdu_header: &ApduHeader,
    data: &[u8],
    sign_tx_context: &mut SignTxContext,
    tx_reviewer: &mut TxReviewer,
) -> Result<(), ErrorCode> {
    match apdu_header.p1 {
        0 if data.len() < 21 => Err(ErrorCode::BadLen), // 20 bytes path + 1 byte token size
        0 => {
            sign_tx_context.init(data)?;
            let token_size = data[20];
            if token_size > MAX_TOKEN_SIZE {
                return Err(ErrorCode::InvalidTokenSize);
            }
            let tx_data_index: usize = 21 + TOKEN_METADATA_SIZE * (token_size as usize);
            if data.len() < tx_data_index + 3 {
                return Err(ErrorCode::BadLen);
            }
            let tx_data = &data[tx_data_index..];
            let is_tx_execute_script = tx_data[2] == 0x01;
            if is_tx_execute_script {
                check_blind_signing()?;
            }
            let token_metadata = &data[21..tx_data_index];
            check_token_metadata(token_size, token_metadata)?;
            tx_reviewer.init(is_tx_execute_script, token_metadata)?;
            sign_tx_context.handle_data(apdu_header, tx_data, tx_reviewer)
        }
        1 => sign_tx_context.handle_data(apdu_header, data, tx_reviewer),
        _ => Err(ErrorCode::BadP1P2),
    }
}

fn check_token_metadata(token_size: u8, token_metadata: &[u8]) -> Result<(), ErrorCode> {
    for i in 0..token_size {
        let version_index = (i as usize) * TOKEN_METADATA_SIZE;
        if token_metadata[version_index] != 0 {
            return Err(ErrorCode::InvalidMetadataVersion);
        }
    }
    Ok(())
}
