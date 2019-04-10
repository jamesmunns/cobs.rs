#![cfg_attr(not(feature = "use_std"), no_std)]

// In the future, don't do this.
mod dec;
mod enc;
pub use crate::dec::*;
pub use crate::enc::*;

/// Calculates the maximum possible size of an encoded message given the length
/// of the source message. This may be useful for calculating how large the
/// `dest` buffer needs to be in the encoding functions.
pub fn max_encoding_length(source_len: usize) -> usize {
    source_len + (source_len / 254) + if source_len % 254 > 0 { 1 } else { 0 }
}
