#[macro_export]
macro_rules! fixed_bytes {
    ($struct_name:ident, $encoded_length:expr) => {
        #[cfg_attr(test, derive(Debug, PartialEq))]
        pub struct $struct_name(pub [u8; $encoded_length]);

        impl $struct_name {
            const ENCODED_LENGTH: usize = $encoded_length;

            pub fn from_bytes(bytes: [u8; $encoded_length]) -> Self {
                $struct_name(bytes)
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self([0; $encoded_length])
            }
        }

        impl RawDecoder for $struct_name {
            fn step_size(&self) -> u16 {
                1
            }

            fn decode<'a>(
                &mut self,
                buffer: &mut Buffer<'a>,
                stage: &DecodeStage,
            ) -> DecodeResult<DecodeStage> {
                let remain = $struct_name::ENCODED_LENGTH - (stage.index as usize);
                let mut idx: usize = 0;
                while !buffer.is_empty() && idx < remain {
                    self.0[(stage.index as usize) + idx] = buffer.next_byte().unwrap();
                    idx += 1;
                }
                let new_index = (stage.index as usize) + idx;
                if new_index == $struct_name::ENCODED_LENGTH {
                    Ok(DecodeStage::COMPLETE)
                } else {
                    Ok(DecodeStage {
                        step: stage.step,
                        index: new_index as u16,
                    })
                }
            }
        }
    };
}
