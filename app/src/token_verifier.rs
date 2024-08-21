use crate::{
    blake2b_hasher::{Blake2bHash, Blake2bHasher, BLAKE2B_HASH_SIZE},
    error_code::ErrorCode,
    handler::TOKEN_METADATA_SIZE,
};

// b3380866c595544781e9da0ccd79399de8878abfb0bf40545b57a287387d419d
const TOKEN_MERKLE_ROOT: Blake2bHash = [
    0xb3, 0x38, 0x08, 0x66, 0xc5, 0x95, 0x54, 0x47, 0x81, 0xe9, 0xda, 0x0c, 0xcd, 0x79, 0x39, 0x9d,
    0xe8, 0x87, 0x8a, 0xbf, 0xb0, 0xbf, 0x40, 0x54, 0x5b, 0x57, 0xa2, 0x87, 0x38, 0x7d, 0x41, 0x9d,
];
const PROOF_PREFIX_LENGTH: usize = 2;

// `TokenVerifier` is a streaming token proof verifier that receives proof data and calculates the hash
// After receiving all the proof data, it compares the hash with the `TOKEN_MERKLE_ROOT` to verify if the token is valid
#[derive(Default, Copy, Clone)]
pub struct TokenVerifier {
    proof_size: usize,
    hash: Blake2bHash,
}

pub fn hash_pair(a: &Blake2bHash, b: &Blake2bHash) -> Result<Blake2bHash, ErrorCode> {
    let mut hasher = Blake2bHasher::new();
    if a < b {
        hasher.update(&a[..])?;
        hasher.update(&b[..])?;
    } else {
        hasher.update(&b[..])?;
        hasher.update(&a[..])?;
    }
    hasher.finalize()
}

impl TokenVerifier {
    pub fn new(data: &[u8]) -> Result<TokenVerifier, ErrorCode> {
        let prefix_length = TOKEN_METADATA_SIZE + PROOF_PREFIX_LENGTH;
        if data.len() < prefix_length {
            return Err(ErrorCode::BadLen);
        }

        let encoded_token = &data[..TOKEN_METADATA_SIZE];
        let proof_size =
            ((data[TOKEN_METADATA_SIZE] as usize) << 8) | (data[TOKEN_METADATA_SIZE + 1] as usize);
        check_proof_size(proof_size)?;

        let proof = &data[prefix_length..];
        check_proof_size(proof.len())?;

        let mut verifier = TokenVerifier {
            proof_size,
            hash: Blake2bHasher::hash(encoded_token)?,
        };
        verifier.update(proof)?;
        Ok(verifier)
    }

    // update the hash when receiving token proof data
    pub fn on_proof(&mut self, proof: &[u8]) -> Result<(), ErrorCode> {
        check_proof_size(proof.len())?;
        self.update(proof)
    }

    fn update(&mut self, proof: &[u8]) -> Result<(), ErrorCode> {
        if self.proof_size < proof.len() {
            return Err(ErrorCode::InvalidTokenProofSize);
        }

        let mut index: usize = 0;
        while index < proof.len() {
            let sibling: Blake2bHash = proof[index..(index + BLAKE2B_HASH_SIZE)]
                .try_into()
                .unwrap();
            self.hash = hash_pair(&self.hash, &sibling)?;
            index += BLAKE2B_HASH_SIZE
        }
        self.proof_size -= proof.len();
        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        self.proof_size == 0
    }

    #[inline]
    pub fn is_token_valid(&self) -> bool {
        assert!(self.is_complete());
        self.hash == TOKEN_MERKLE_ROOT
    }
}

fn check_proof_size(size: usize) -> Result<(), ErrorCode> {
    if size % BLAKE2B_HASH_SIZE != 0 {
        Err(ErrorCode::InvalidTokenProofSize)
    } else {
        Ok(())
    }
}
