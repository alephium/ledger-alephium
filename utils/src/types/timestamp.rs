use crate::buffer::Buffer;
use crate::decode::*;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
pub struct TimeStamp(pub u64);

impl TimeStamp {
    const ENCODED_LENGTH: usize = 8;
}

impl Reset for TimeStamp {
    fn reset(&mut self) {
        self.0 = 0;
    }
}

impl RawDecoder for TimeStamp {
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<'a>(
        &mut self,
        buffer: &mut Buffer<'a>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        let remain = Self::ENCODED_LENGTH - (stage.index as usize);
        let mut idx: usize = 0;
        while !buffer.is_empty() && idx < remain {
            let byte = buffer.next_byte().unwrap();
            self.0 |= ((byte & 0xff) as u64) << ((remain - 1 - idx) * 8);
            idx += 1;
        }
        let new_index = (stage.index as usize) + idx;
        if new_index == Self::ENCODED_LENGTH {
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
    extern crate std;

    use crate::buffer::Buffer;
    use crate::decode::{new_decoder, Decoder};
    use rand::Rng;

    use super::TimeStamp;

    fn gen_data() -> ([u8; 8], u64) {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 8];
        rng.fill(&mut bytes[..]);
        let number = u64::from_be_bytes(bytes);
        (bytes, number)
    }

    #[test]
    fn test_decode_timestamp() {
        let (bytes, number) = gen_data();
        let steps = [1, 2, 4, 8];

        for step in steps {
            let mut decoder = new_decoder::<TimeStamp>();
            let mut buffer = Buffer::new(&bytes[..0]).unwrap();
            assert_eq!(decoder.decode(&mut buffer), Ok(None));

            let end_index = bytes.len() - step;
            for i in (0..end_index).step_by(step) {
                let to = std::cmp::min(end_index, i + step);
                let mut buffer = Buffer::new(&bytes[i..to]).unwrap();
                assert_eq!(decoder.decode(&mut buffer), Ok(None));
            }

            let mut buffer = Buffer::new(&bytes[(bytes.len() - step)..]).unwrap();
            assert_eq!(decoder.decode(&mut buffer), Ok(Some(&TimeStamp(number))));
        }
    }
}
