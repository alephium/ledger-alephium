use crate::buffer::Buffer;
use crate::decode::*;
use crate::types::compact_integer::*;

#[cfg_attr(test, derive(Debug))]
pub struct I256 {
    bytes: [u8; 33], // TODO: improve this
}

impl Default for I256 {
    fn default() -> Self {
        I256 { bytes: [0u8; 33] }
    }
}

impl PartialEq for I256 {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl I256 {
    #[inline]
    pub fn get_length(&self) -> usize {
        decode_length(self.bytes[0])
    }

    #[inline]
    pub fn is_fixed_size(&self) -> bool {
        is_fixed_size(self.bytes[0])
    }
}

impl RawDecoder for I256 {
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if buffer.is_empty() {
            return Ok(DecodeStage { ..*stage });
        }
        let from_index = if stage.index == 0 {
            self.bytes[0] = buffer.next_byte().unwrap();
            1
        } else {
            stage.index
        };
        let length = self.get_length();
        let mut idx = 0;
        while !buffer.is_empty() && idx < (length - (from_index as usize)) {
            self.bytes[(from_index as usize) + idx] = buffer.next_byte().unwrap();
            idx += 1;
        }
        let new_index = (from_index as usize) + idx;
        if new_index == length {
            Ok(DecodeStage::COMPLETE)
        } else {
            Ok(DecodeStage {
                step: stage.step,
                index: new_index as u16,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    extern crate std;

    use super::*;
    use crate::types::i32::tests::random_usize;
    use crate::types::u256::tests::hex_to_bytes;

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

        for item in arrays {
            let bytes = hex_to_bytes(item).unwrap();

            {
                let mut decoder = new_decoder::<I256>();
                let mut buffer = Buffer::new(&bytes).unwrap();
                let result = decoder.decode(&mut buffer).unwrap().unwrap();
                let length = result.get_length();
                assert_eq!(&bytes, &result.bytes[..length]);
                assert!(decoder.stage.is_complete());
            }

            let mut length: usize = 0;
            let mut decoder = new_decoder::<I256>();

            while length < bytes.len() {
                let remain = bytes.len() - length;
                let size = random_usize(0, remain);
                let mut buffer = Buffer::new(&bytes[length..(length + size)]).unwrap();
                length += size;

                let result = decoder.decode(&mut buffer).unwrap();
                if length == bytes.len() {
                    let result = result.unwrap();
                    let length = result.get_length();
                    assert_eq!(&bytes, &result.bytes[..length]);
                    assert!(decoder.stage.is_complete());
                } else {
                    assert_eq!(result, None);
                    assert_eq!(decoder.stage.index as usize, length);
                }
            }
        }
    }
}
