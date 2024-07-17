use crate::buffer::{Buffer, Writable};
use crate::decode::*;
use crate::fixed_size_integer;

fixed_size_integer!(TimeStamp, 8, u64);

#[cfg(test)]
mod tests {
    extern crate std;

    use crate::buffer::Buffer;
    use crate::decode::{new_decoder, Decoder};
    use crate::TempData;
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
        let mut temp_data = TempData::new();

        for step in steps {
            let mut decoder = new_decoder::<TimeStamp>();
            let mut buffer = Buffer::new(&bytes[..0], &mut temp_data);
            assert_eq!(decoder.decode(&mut buffer), Ok(None));

            let end_index = bytes.len() - step;
            for i in (0..end_index).step_by(step) {
                let to = std::cmp::min(end_index, i + step);
                let mut buffer = Buffer::new(&bytes[i..to], &mut temp_data);
                assert_eq!(decoder.decode(&mut buffer), Ok(None));
            }

            let mut buffer = Buffer::new(&bytes[(bytes.len() - step)..], &mut temp_data);
            assert_eq!(decoder.decode(&mut buffer), Ok(Some(&TimeStamp(number))));
        }
    }
}
