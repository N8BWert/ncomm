//!
//! Utility Packing and Unpacking Methods Necessary for Data
//! Sent over Some Network.
//!

/// An error from attempting to pack data into a buffer or from
/// attempting to unpack data from a slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackingError {
    /// The buffer to pack or unpack data from cannot be used as
    /// the data will not fit in the buffer.
    InvalidBufferSize,
}

/// Trait implemented by data to be sent over network boundaries.
pub trait Packable: Sized {
    /// Get the minimum necessary length of a buffer to pack this data
    /// into.
    fn len() -> usize;

    /// Pack a given piece of data into a given buffer.
    ///
    /// Note: this format of pack was utilized to make this trait
    /// compatible with both std and no_std targets (specifically
    /// for no_std targets without `alloc`)
    fn pack(self, buffer: &mut [u8]) -> Result<(), PackingError>;

    /// Unpack a given piece of data from an array of bytes
    fn unpack(data: &[u8]) -> Result<Self, PackingError>;
}

#[cfg(feature = "little-endian")]
macro_rules! packable_primitive {
    ($primitive_name: ident, $length: literal) => {
        impl Packable for $primitive_name {
            fn len() -> usize {
                $length as usize
            }

            fn pack(self, buffer: &mut [u8]) -> Result<(), PackingError> {
                if buffer.len() < Self::len() {
                    return Err(PackingError::InvalidBufferSize);
                }

                buffer[..Self::len()].copy_from_slice(&self.to_le_bytes()[..]);
                Ok(())
            }

            fn unpack(data: &[u8]) -> Result<Self, PackingError> {
                if data.len() < Self::len() {
                    return Err(PackingError::InvalidBufferSize);
                }

                Ok(Self::from_le_bytes(data[..Self::len()].try_into().unwrap()))
            }
        }
    };
}

#[cfg(not(feature = "little-endian"))]
macro_rules! packable_primitive {
    ($primitive_name: ident, $length: literal) => {
        impl Packable for $primitive_name {
            fn len() -> usize {
                $length as usize
            }

            fn pack(self, buffer: &mut [u8]) -> Result<(), PackingError> {
                if buffer.len() < Self::len() {
                    return Err(PackingError::InvalidBufferSize);
                }

                buffer[..Self::len()].copy_from_slice(&self.to_be_bytes()[..]);
                Ok(())
            }

            fn unpack(data: &[u8]) -> Result<Self, PackingError> {
                if data.len() < Self::len() {
                    return Err(PackingError::InvalidBufferSize);
                }

                Ok(Self::from_be_bytes(data[..Self::len()].try_into().unwrap()))
            }
        }
    };
}

packable_primitive!(u8, 1);
packable_primitive!(u16, 2);
packable_primitive!(u32, 4);
packable_primitive!(u64, 8);
packable_primitive!(u128, 16);
packable_primitive!(usize, 8);
packable_primitive!(i8, 1);
packable_primitive!(i16, 2);
packable_primitive!(i32, 4);
packable_primitive!(i64, 8);
packable_primitive!(i128, 16);
packable_primitive!(isize, 8);
packable_primitive!(f32, 4);
packable_primitive!(f64, 8);

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_primitive_packing {
        ($primitive: ident, $buffer_length: literal, $value: literal, $test_name: ident) => {
            #[test]
            fn $test_name() {
                let mut buffer = [0u8; $buffer_length as usize];
                assert!($value.pack(&mut buffer).is_ok());
                println!("buffer: {:?}", buffer);
                assert_eq!($value, $primitive::unpack(&buffer).unwrap());
            }
        };
    }

    test_primitive_packing!(u8, 1, 129u8, test_u8_packing);
    test_primitive_packing!(u16, 2, 129u16, test_u16_packing);
    test_primitive_packing!(u32, 4, 129u32, test_u32_packing);
    test_primitive_packing!(u64, 8, 129u64, test_u64_packing);
    test_primitive_packing!(u128, 16, 129u128, test_u128_packing);
    test_primitive_packing!(usize, 16, 129usize, test_usize_packing);
    test_primitive_packing!(i8, 1, 15i8, test_i8_packing);
    test_primitive_packing!(i16, 2, 129i16, test_i16_packing);
    test_primitive_packing!(i32, 4, 129i32, test_i32_packing);
    test_primitive_packing!(i64, 8, 129i64, test_i64_packing);
    test_primitive_packing!(i128, 16, 129i128, test_i128_packing);
    test_primitive_packing!(isize, 16, 129isize, test_isize_packing);
    test_primitive_packing!(f32, 4, 2.01f32, test_f32_packing);
    test_primitive_packing!(f64, 8, 2.01f64, test_f64_packing);
}