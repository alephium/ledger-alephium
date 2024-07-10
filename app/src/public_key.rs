use ledger_device_sdk::ecc::SeedDerive;
use ledger_device_sdk::ecc::{ECPublicKey, Secp256k1};
use ledger_device_sdk::io::Reply;
use utils::{djb_hash, xor_bytes};
use crate::blake2b_hasher::Blake2bHasher;
use crate::debug::print::{println, println_slice};
use crate::error_code::ErrorCode;

pub const TOTAL_NUMBER_OF_GROUPS: u8 = 4;

fn check_group(group_num: u8, target_group: u8) -> Result<(), Reply> {
  if group_num == 0 && target_group == 0 {
      return Ok(());
  }
  if target_group >= group_num || group_num != TOTAL_NUMBER_OF_GROUPS {
      return Err(ErrorCode::BadP1P2.into());
  }
  return Ok(());
}

pub fn derive_pub_key(
  path: &mut [u32],
  group_num: u8,
  target_group: u8
) -> Result<(ECPublicKey<65, 'W'>, u32), Reply> {
  check_group(group_num, target_group)?;
  if group_num == 0 {
    let pub_key = derive_pub_key_by_path(&path)?;
    Ok((pub_key, path[path.len() - 1]))
  } else {
    derive_pub_key_for_group(path, group_num, target_group)
  }
}

fn derive_pub_key_by_path(path: &[u32]) -> Result<ECPublicKey<65, 'W'>, Reply> {
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
      println("path");
      println_slice::<8>(&path.last().unwrap().to_be_bytes());
      let pk = derive_pub_key_by_path(path)?;
      if get_pub_key_group(pk.as_ref(), group_num) == target_group {
          return Ok((pk, path[path.len() - 1]));
      }
      path[path.len() - 1] += 1;
  }
}

fn get_pub_key_group(pub_key: &[u8], group_num: u8) -> u8 {
  assert!(pub_key.len() == 65);
  println("pub_key 65");
  println_slice::<130>(pub_key);
  let mut compressed = [0_u8; 33];
  compressed[1..33].copy_from_slice(&pub_key[1..33]);
  if pub_key.last().unwrap() % 2 == 0 {
      compressed[0] = 0x02
  } else {
      compressed[0] = 0x03
  }
  println("compressed");
  println_slice::<66>(&compressed);

  let pub_key_hash = Blake2bHasher::hash(&compressed).unwrap();
  println("blake2b done");
  let script_hint = djb_hash(&pub_key_hash) | 1;
  println("hint done");
  let group_index = xor_bytes(script_hint);
  println("pub key hash");
  println_slice::<64>(&pub_key_hash);
  println("script hint");
  println_slice::<8>(&script_hint.to_be_bytes());
  println("group index");
  println_slice::<2>(&[group_index]);

  group_index % group_num
}