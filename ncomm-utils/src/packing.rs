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