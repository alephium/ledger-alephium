use crate::buffer::Buffer;
use crate::decode::*;
use crate::fixed_size_bytes;

fixed_size_bytes!(Checksum, 4);

pub trait Checksumable {
    fn calc_checksum(&self) -> Checksum;
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Checksummed<T> {
    pub value: T,
    pub checksum: Checksum,
}

impl<T: Sized + Default> Default for Checksummed<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            checksum: Checksum::default(),
        }
    }
}

impl<T: Sized + Reset> Reset for Checksummed<T> {
    fn reset(&mut self) {
        self.value.reset();
        self.checksum.reset();
    }
}

impl<T: RawDecoder + Sized + Checksumable> RawDecoder for Checksummed<T> {
    fn step_size(&self) -> u16 {
        self.value.step_size() + 1
    }

    fn decode<W: crate::buffer::Writable>(
        &mut self,
        buffer: &mut Buffer<'_, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        match stage.step {
            step if step < self.value.step_size() => self.value.decode(buffer, stage),
            step if step < self.step_size() => {
                let result = self.checksum.decode(buffer, stage);
                match result {
                    Ok(stage) if stage.is_complete() => {
                        if self.checksum.0 == self.value.calc_checksum().0 {
                            Ok(DecodeStage::COMPLETE)
                        } else {
                            Err(DecodeError::InvalidData)
                        }
                    }
                    _ => result,
                }
            }
            _ => Err(DecodeError::InternalError),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        buffer::Buffer,
        decode::{new_decoder, DecodeError, Decoder},
        types::{u256::tests::hex_to_bytes, Checksummed, PublicKeyLike},
        TempData,
    };

    #[test]
    fn test_checksummed() {
        let test_vectors = [
            hex_to_bytes(
                "00e7e379a63b39fef2cb42e5cf88bef4e78c9bd2ec0de6f28a0a0ba91937e7260624dfe0ed4d",
            )
            .unwrap(),
            hex_to_bytes(
                "011c6465f5fb36f036b9915699fe467b14bcbb260dc004f3f68fdedb37101b48edc748875be0",
            )
            .unwrap(),
            hex_to_bytes(
                "0262e9a4736b18738ccb1dbe6d0b05872c6019faf21d269f00d1744363a86a8aedb76dc73c",
            )
            .unwrap(),
            hex_to_bytes(
                "038652277ddfd3b919e0d4415fffa9cc0e05838541ecc9bd0007bf4716604fdcdd17e7bc6f40",
            )
            .unwrap(),
        ];
        for bytes in test_vectors {
            let mut temp_data = TempData::new();
            let mut buffer0 = Buffer::new(&bytes, &mut temp_data);
            let mut decoder = new_decoder::<Checksummed<PublicKeyLike>>();
            let result0 = decoder.decode(&mut buffer0).unwrap();
            assert!(result0.is_some());

            let bytes_with_invalid_checksum = [&bytes[..bytes.len() - 1], &[0u8]].concat();
            let mut buffer1 = Buffer::new(&bytes_with_invalid_checksum, &mut temp_data);
            decoder.reset();
            let result1 = decoder.decode(&mut buffer1);
            assert_eq!(result1, Err(DecodeError::InvalidData));
        }
    }
}
