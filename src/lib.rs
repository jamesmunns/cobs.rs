#![cfg_attr(not(feature = "use_std"), no_std)]

// In the future, don't do this.
mod dec;
mod enc;
pub use crate::dec::*;
pub use crate::enc::*;

/// Calculates the maximum overhead when encoding a message with the given length.
/// The overhead is a maximum of [n/254] bytes (one in 254 bytes) rounded up.
pub const fn max_encoding_overhead(source_len: usize) -> usize {
    (source_len + 254 - 1) / 254
}

/// Calculates the maximum possible size of an encoded message given the length
/// of the source message. This may be useful for calculating how large the
/// `dest` buffer needs to be in the encoding functions.
pub const fn max_encoding_length(source_len: usize) -> usize {
    source_len + max_encoding_overhead(source_len)
}

#[cfg(test)]
mod tests {
    use crate::{max_encoding_length, max_encoding_overhead};

    // Usable in const context
    const ENCODED_BUF: [u8; max_encoding_length(5)] = [0; max_encoding_length(5)];

    #[test]
    fn test_buf_len() {
        assert_eq!(ENCODED_BUF.len(), 6);
    }

    #[test]
    fn test_overhead_empty() {
        assert_eq!(max_encoding_overhead(0), 0);
    }

    #[test]
    fn test_overhead_one() {
        assert_eq!(max_encoding_overhead(1), 1);
    }

    #[test]
    fn test_overhead_larger() {
        assert_eq!(max_encoding_overhead(253), 1);
        assert_eq!(max_encoding_overhead(254), 1);
    }

    #[test]
    fn test_overhead_two() {
        assert_eq!(max_encoding_overhead(255), 2);
    }
}
