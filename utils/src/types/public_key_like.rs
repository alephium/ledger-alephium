use super::{ED25519PubKey, SecP256K1PubKey, SecP256R1PubKey};
use crate::buffer::{Buffer, Writable};
use crate::types::{Checksum, Checksumable};
use crate::{decode::*, djb_hash_with_prefix};

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub enum PublicKeyLike {
    SecP256K1(SecP256K1PubKey),
    SecP256R1(SecP256R1PubKey),
    ED25519(ED25519PubKey),
    WebAuthn(SecP256R1PubKey),
    #[default]
    Unknown,
}

impl Checksumable for PublicKeyLike {
    fn calc_checksum(&self) -> Checksum {
        let hash: [u8; 4] = djb_hash_with_prefix(self.get_type(), self.key_bytes()).to_be_bytes();
        Checksum(hash)
    }
}

impl Reset for PublicKeyLike {
    fn reset(&mut self) {
        *self = Self::Unknown;
    }
}

impl PublicKeyLike {
    fn from_type(tpe: u8) -> Option<Self> {
        match tpe {
            0 => Some(PublicKeyLike::SecP256K1(SecP256K1PubKey::default())),
            1 => Some(PublicKeyLike::SecP256R1(SecP256R1PubKey::default())),
            2 => Some(PublicKeyLike::ED25519(ED25519PubKey::default())),
            3 => Some(PublicKeyLike::WebAuthn(SecP256R1PubKey::default())),
            _ => None,
        }
    }

    pub fn get_type(&self) -> u8 {
        match self {
            PublicKeyLike::SecP256K1(_) => 0,
            PublicKeyLike::SecP256R1(_) => 1,
            PublicKeyLike::ED25519(_) => 2,
            PublicKeyLike::WebAuthn(_) => 3,
            _ => 0xff, // dead branch
        }
    }

    pub fn key_bytes(&self) -> &[u8] {
        match self {
            PublicKeyLike::SecP256K1(key) => &key.0,
            PublicKeyLike::SecP256R1(key) => &key.0,
            PublicKeyLike::ED25519(key) => &key.0,
            PublicKeyLike::WebAuthn(key) => &key.0,
            PublicKeyLike::Unknown => &[],
        }
    }
}

impl RawDecoder for PublicKeyLike {
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<W: Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if buffer.is_empty() {
            return Ok(DecodeStage { ..*stage });
        }
        if let PublicKeyLike::Unknown = self {
            let tpe = buffer.consume_byte().unwrap();
            let result = PublicKeyLike::from_type(tpe);
            if result.is_none() {
                return Err(DecodeError::InvalidData);
            }
            *self = result.unwrap();
        };
        match self {
            PublicKeyLike::SecP256K1(pub_key) => pub_key.decode(buffer, stage),
            PublicKeyLike::SecP256R1(pub_key) => pub_key.decode(buffer, stage),
            PublicKeyLike::ED25519(pub_key) => pub_key.decode(buffer, stage),
            PublicKeyLike::WebAuthn(pub_key) => pub_key.decode(buffer, stage),
            PublicKeyLike::Unknown => Err(DecodeError::InternalError),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use crate::types::byte32::tests::gen_bytes;
    use crate::types::test_utils::test_decode;
    use crate::types::{ED25519PubKey, PublicKeyLike, SecP256K1PubKey, SecP256R1PubKey};

    #[test]
    fn test_decode_secp256k1() {
        let pub_key = gen_bytes(33, 33);
        let key = PublicKeyLike::SecP256K1(SecP256K1PubKey::from_bytes(
            pub_key.clone().try_into().unwrap(),
        ));
        test_decode(0u8, pub_key, Some(&key), None);
    }

    #[test]
    fn test_decode_secp256r1() {
        let pub_key = gen_bytes(33, 33);
        let key = PublicKeyLike::SecP256R1(SecP256R1PubKey::from_bytes(
            pub_key.clone().try_into().unwrap(),
        ));
        test_decode(1u8, pub_key, Some(&key), None);
    }

    #[test]
    fn test_decode_ed25519() {
        let pub_key = gen_bytes(32, 32);
        let key = PublicKeyLike::ED25519(ED25519PubKey::from_bytes(
            pub_key.clone().try_into().unwrap(),
        ));
        test_decode(2u8, pub_key, Some(&key), None);
    }

    #[test]
    fn test_decode_webauthn() {
        let pub_key = gen_bytes(33, 33);
        let key = PublicKeyLike::WebAuthn(SecP256R1PubKey::from_bytes(
            pub_key.clone().try_into().unwrap(),
        ));
        test_decode(3u8, pub_key, Some(&key), None);
    }
}
