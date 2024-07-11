use crate::buffer::{Buffer, Writable};
use crate::decode::*;
use crate::fixed_size_integer;

fixed_size_integer!(MethodSelector, 4, i32);

#[cfg(test)]
pub mod tests {
    extern crate std;

    use crate::buffer::Buffer;
    use crate::decode::{new_decoder, Decoder};
    use crate::TempData;
    use rand::Rng;

    use super::MethodSelector;

    fn gen_data() -> ([u8; 4], i32) {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 4];
        rng.fill(&mut bytes[..]);
        let number = i32::from_be_bytes(bytes);
        (bytes, number)
    }

    #[test]
    fn test_decode_method_selector() {
        let (bytes, number) = gen_data();
        let steps = [1, 2, 3, 4];
        let mut temp_data = TempData::new();

        for step in steps {
            let mut decoder = new_decoder::<MethodSelector>();
            let mut buffer = Buffer::new(&bytes[..0], &mut temp_data).unwrap();
            assert_eq!(decoder.decode(&mut buffer), Ok(None));

            let end_index = bytes.len() - step;
            for i in (0..end_index).step_by(step) {
                let to = std::cmp::min(end_index, i + step);
                let mut buffer = Buffer::new(&bytes[i..to], &mut temp_data).unwrap();
                assert_eq!(decoder.decode(&mut buffer), Ok(None));
            }

            let mut buffer = Buffer::new(&bytes[(bytes.len() - step)..], &mut temp_data).unwrap();
            assert_eq!(
                decoder.decode(&mut buffer),
                Ok(Some(&MethodSelector(number)))
            );
        }
    }
}
