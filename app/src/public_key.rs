use core::str::from_utf8;
use crate::blake2b_hasher::{Blake2bHasher, BLAKE2B_HASH_SIZE};
use crate::error_code::ErrorCode;
use ledger_device_sdk::ecc::SeedDerive;
use ledger_device_sdk::ecc::{ECPublicKey, Secp256k1};
use ledger_device_sdk::io::Reply;
use utils::base58::base58_encode_inputs;
use utils::{djb_hash, xor_bytes};

pub const TOTAL_NUMBER_OF_GROUPS: u8 = 4;

fn check_group(group_num: u8, target_group: u8) -> Result<(), Reply> {
    if group_num == 0 && target_group == 0 {
        return Ok(());
    }
    if target_group >= group_num || group_num != TOTAL_NUMBER_OF_GROUPS {
        return Err(ErrorCode::BadP1P2.into());
    }
    Ok(())
}

pub fn derive_pub_key(
    path: &mut [u32],
    group_num: u8,
    target_group: u8,
) -> Result<(ECPublicKey<65, 'W'>, u32), Reply> {
    check_group(group_num, target_group)?;
    if group_num == 0 {
        let pub_key = derive_pub_key_by_path(path)?;
        Ok((pub_key, path[path.len() - 1]))
    } else {
        derive_pub_key_for_group(path, group_num, target_group)
    }
}

pub fn derive_pub_key_by_path(path: &[u32]) -> Result<ECPublicKey<65, 'W'>, Reply> {
    let pk = Secp256k1::derive_from_path(path)
        .public_key()
        .map_err(|x| Reply(0x6eu16 | (x as u16 & 0xff)))?;
    Ok(pk)
}

fn derive_pub_key_for_group(
    path: &mut [u32],
    group_num: u8,
    target_group: u8,
) -> Result<(ECPublicKey<65, 'W'>, u32), Reply> {
    loop {
        let pk = derive_pub_key_by_path(path)?;
        if get_pub_key_group(pk.as_ref(), group_num) == target_group {
            return Ok((pk, path[path.len() - 1]));
        }
        path[path.len() - 1] += 1;
    }
}

pub fn hash_of_public_key(pub_key: &[u8]) -> [u8; BLAKE2B_HASH_SIZE] {
    assert!(pub_key.len() == 65);
    let mut compressed = [0_u8; 33];
    compressed[1..33].copy_from_slice(&pub_key[1..33]);
    if pub_key.last().unwrap() % 2 == 0 {
        compressed[0] = 0x02
    } else {
        compressed[0] = 0x03
    }

    Blake2bHasher::hash(&compressed).unwrap()
}

fn get_pub_key_group(pub_key: &[u8], group_num: u8) -> u8 {
    let pub_key_hash = hash_of_public_key(pub_key);
    let script_hint = djb_hash(&pub_key_hash) | 1;
    let group_index = xor_bytes(script_hint);
    group_index % group_num
}

pub fn sign_hash(path: &[u32], message: &[u8]) -> Result<([u8; 72], u32, u32), ErrorCode> {
    Secp256k1::derive_from_path(path)
        .deterministic_sign(message)
        .map_err(|_| ErrorCode::TxSigningFailed)
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

    pub fn from_pub_key(pub_key: &ECPublicKey<65, 'W'>) -> Result<Self, ErrorCode> {
        let mut bytes = [0u8; 46];
        let public_key_hash = hash_of_public_key(pub_key.as_ref());
        let device_address = to_base58_address(0u8, &public_key_hash, &mut bytes)?;
        let length = device_address.len();
        Ok(Self { bytes, length })
    }

    pub fn get_address_str(&self) -> Result<&str, ErrorCode> {
        from_utf8(&self.bytes[..self.length]).map_err(|_| ErrorCode::InternalError)
    }

    pub fn eq(&self, addr: &[u8]) -> bool {
        &self.bytes[..self.length] == addr
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
