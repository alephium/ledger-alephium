use super::BigInt;
use crate::buffer::{Buffer, Writable};
use crate::decode::*;

#[cfg_attr(test, derive(Debug))]
#[derive(Default, PartialEq)]
pub struct I256(pub BigInt);

impl Reset for I256 {
    fn reset(&mut self) {
        self.0.reset()
    }
}

impl RawDecoder for I256 {
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<'a, W: Writable>(
        &mut self,
        buffer: &mut Buffer<'a, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        self.0.decode(buffer, stage)
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    extern crate std;

    use super::*;
    use crate::types::i32::tests::random_usize;
    use crate::types::u256::tests::hex_to_bytes;
    use crate::TempData;

    #[test]
    fn test_decode_i256() {
        let arrays = [
            "00",
            "01",
            "02",
            "3f",
            "3e",
            "1e",
            "1f",
            "4020",
            "4021",
            "4022",
            "5ffe",
            "5fff",
            "80002000",
            "80002001",
            "80002002",
            "9ffffffe",
            "9fffffff",
            "c020000000",
            "c020000001",
            "c020000002",
            "c07fffffff",
            "c080000000",
            "c03fffffff",
            "c0c0000000",
            "dc8000000000000000000000000000000000000000000000000000000000000000",
            "dc7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "c5010000000000000000",
            "c5ff0000000000000000",
            "cd0100000000000000000000000000000000",
            "cdff00000000000000000000000000000000",
            "d501000000000000000000000000000000000000000000000000",
            "d5ff000000000000000000000000000000000000000000000000",
        ];

        let mut temp_data = TempData::new();
        for item in arrays {
            let bytes = hex_to_bytes(item).unwrap();

            {
                let mut decoder = new_decoder::<I256>();
                let mut buffer = Buffer::new(&bytes, &mut temp_data).unwrap();
                let result = decoder.decode(&mut buffer).unwrap().unwrap();
                let length = result.0.get_length();
                assert_eq!(&bytes, &result.0.bytes[..length]);
                assert!(decoder.stage.is_complete());
            }

            let mut length: usize = 0;
            let mut decoder = new_decoder::<I256>();

            while length < bytes.len() {
                let remain = bytes.len() - length;
                let size = random_usize(0, remain);
                let mut buffer =
                    Buffer::new(&bytes[length..(length + size)], &mut temp_data).unwrap();
                length += size;

                let result = decoder.decode(&mut buffer).unwrap();
                if length == bytes.len() {
                    let result = result.unwrap();
                    let length = result.0.get_length();
                    assert_eq!(&bytes, &result.0.bytes[..length]);
                    assert!(decoder.stage.is_complete());
                } else {
                    assert_eq!(result, None);
                    assert_eq!(decoder.stage.index as usize, length);
                }
            }
        }
    }
}
