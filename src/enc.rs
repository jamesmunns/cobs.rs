#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[error("out of bounds error during encoding")]
pub struct DestBufTooSmallError;

/// The [`EncoderState`] is used to track the current state of a streaming encoder. This struct
/// does not contain the output buffer (or a reference to one), and can be used when streaming the
/// encoded output to a custom data type
///
/// **IMPORTANT NOTE**: When implementing a custom streaming encoder,
/// the [`EncoderState`] state machine assumes that the output buffer
/// **ALREADY** contains a single placeholder byte, and no other bytes.
/// This placeholder byte will be later modified with the first distance
/// to the next header/zero byte.
#[derive(Clone, Debug)]
pub struct EncoderState {
    code_idx: usize,
    num_bt_sent: u8,
    offset_idx: u8,
}

/// [`PushResult`] is used to represent the changes to an (encoded)
/// output data buffer when an unencoded byte is pushed into [`EncoderState`].
pub enum PushResult {
    /// The returned byte should be placed at the current end of the data buffer
    AddSingle(u8),

    /// The byte at the given index should be replaced with the given byte.
    /// Additionally, a placeholder byte should be inserted at the current
    /// end of the output buffer to be later modified
    ModifyFromStartAndSkip((usize, u8)),

    /// The byte at the given index should be replaced with the given byte.
    /// Then, the last u8 in this tuple should be inserted at the end of the
    /// current output buffer. Finally, a placeholder byte should be inserted at
    /// the current end of the output buffer to be later modified if the encoding process is
    /// not done yet.
    ModifyFromStartAndPushAndSkip((usize, u8, u8)),
}

impl Default for EncoderState {
    /// Create a default initial state representation for a COBS encoder
    fn default() -> Self {
        Self {
            code_idx: 0,
            num_bt_sent: 1,
            offset_idx: 1,
        }
    }
}

impl EncoderState {
    /// Push a single unencoded byte into the encoder state machine
    pub fn push(&mut self, data: u8) -> PushResult {
        if data == 0 {
            let ret = PushResult::ModifyFromStartAndSkip((self.code_idx, self.num_bt_sent));
            self.code_idx += usize::from(self.offset_idx);
            self.num_bt_sent = 1;
            self.offset_idx = 1;
            ret
        } else {
            self.num_bt_sent += 1;
            self.offset_idx += 1;

            if 0xFF == self.num_bt_sent {
                let ret = PushResult::ModifyFromStartAndPushAndSkip((
                    self.code_idx,
                    self.num_bt_sent,
                    data,
                ));
                self.num_bt_sent = 1;
                self.code_idx += usize::from(self.offset_idx);
                self.offset_idx = 1;
                ret
            } else {
                PushResult::AddSingle(data)
            }
        }
    }

    /// Finalize the encoding process for a single message.
    /// The byte at the given index should be replaced with the given value,
    /// and the sentinel value (typically 0u8) must be inserted at the current
    /// end of the output buffer, serving as a framing byte.
    pub fn finalize(self) -> (usize, u8) {
        (self.code_idx, self.num_bt_sent)
    }
}

/// The [`CobsEncoder`] type is used to encode a stream of bytes to a given mutable output slice.
///
/// This is often useful when heap data structures are not available, or when not all message bytes
/// are received at a single point in time.
#[derive(Debug)]
pub struct CobsEncoder<'a> {
    dest: &'a mut [u8],
    dest_idx: usize,
    state: EncoderState,
    might_be_done: bool,
    failed: bool,
}

impl<'a> CobsEncoder<'a> {
    /// Create a new streaming Cobs Encoder.
    ///
    /// `out_buf` should not be an empty slice. Doing so will not panic, but
    /// every call to `push` will fail, and `finalize()` will return `0`.
    pub fn new(out_buf: &'a mut [u8]) -> CobsEncoder<'a> {
        CobsEncoder {
            dest_idx: 1,
            state: EncoderState::default(),
            might_be_done: false,
            // it was never gunna work.
            failed: out_buf.is_empty(),
            dest: out_buf,
        }
    }

    /// Push a slice of data to be encoded
    ///
    /// If this function returns `Err`, all subsequent calls to `push` will also
    /// return an `Err`, and [`Self::finalize()`] will always return 0.
    pub fn push(&mut self, data: &[u8]) -> Result<(), DestBufTooSmallError> {
        // On any failing call to push, fuse the error to avoid misleading output.
        if self.failed {
            return Err(DestBufTooSmallError);
        }
        let res = self.push_inner(data);
        self.failed |= res.is_err();
        res
    }

    #[inline]
    fn push_inner(&mut self, data: &[u8]) -> Result<(), DestBufTooSmallError> {
        // If data is empty, don't update any tracking variables.
        if data.is_empty() {
            return Ok(());
        }

        // TODO: could probably check if this would fit without
        // iterating through all data

        // There was the possibility that the encoding process is done, but more data is pushed
        // instead of a `finalize` call, so the destination index needs to be incremented.
        if self.might_be_done {
            self.dest_idx += 1;
            self.might_be_done = false;
        }

        // Consume the data, byte at a time.
        for (slice_idx, val) in data.iter().enumerate() {
            use PushResult::*;
            match self.state.push(*val) {
                AddSingle(y) => {
                    *self
                        .dest
                        .get_mut(self.dest_idx)
                        .ok_or(DestBufTooSmallError)? = y;
                }
                ModifyFromStartAndSkip((idx, mval)) => {
                    // Modify from start...
                    *self.dest.get_mut(idx).ok_or(DestBufTooSmallError)? = mval;

                    // ...and skip.
                    //
                    // The skip means that we are now making room for the NEXT
                    // placeholder byte. If we're already at/past the end of
                    // `dest`, we wouldn't ever be able to write that back, so
                    // go ahead and bail now.
                    if self.dest_idx >= self.dest.len() {
                        return Err(DestBufTooSmallError);
                    }
                }
                ModifyFromStartAndPushAndSkip((idx, mval, nval1)) => {
                    // Modify from start...
                    *self.dest.get_mut(idx).ok_or(DestBufTooSmallError)? = mval;

                    // ...and push...
                    *self
                        .dest
                        .get_mut(self.dest_idx)
                        .ok_or(DestBufTooSmallError)? = nval1;

                    // ...and (maybe) skip.
                    // Do not increase index if these is the possibility that we are finished.
                    // We've already checked `data.len()` is non-zero.
                    if slice_idx == data.len() - 1 {
                        // If push (with non-empty data) is called again, the index will be incremented.
                        // If finalize is called, there is no need to increment the index.
                        self.might_be_done = true;
                    } else {
                        self.dest_idx += 1;
                    }
                }
            }

            // All branches above require advancing the pointer at least once
            self.dest_idx += 1;
        }

        Ok(())
    }

    /// Complete encoding of the output message. Does NOT terminate the message with the sentinel
    /// value.
    ///
    /// An empty message encodes as `[0x01]`. If an empty output buffer is provided in
    /// [`Self::new()`], that byte cannot be written, and this will return zero.
    ///
    /// You should NOT call `finalize` after a call to [`Self::push`] returned an `Err`.
    /// If you do so, this will return zero.
    pub fn finalize(self) -> usize {
        if self.failed {
            return 0;
        }
        // Get the last index that needs to be fixed
        let (idx, mval) = self.state.finalize();

        // After a final full 254-byte data run, `idx == dest_idx`.
        // The next run's pending code byte was never counted as output,
        // so it must not be written either.
        if idx < self.dest_idx {
            // If the current code index is outside of the destination slice,
            // we cannot write it out. This ONLY occurs if `self.dest` was empty,
            // OR if a call to `Self::push` failed, both of which should be caught
            // by `self.failed`. However, just in case, report `0` if we can't
            // record the final fix.
            if let Some(i) = self.dest.get_mut(idx) {
                *i = mval;
            } else {
                return 0;
            }
        }

        self.dest_idx
    }
}

/// Encodes the `source` buffer into the `dest` buffer.
///
/// This function assumes the typical sentinel value of 0, but does not terminate the encoded
/// message with the sentinel value. This should be done by the caller to ensure proper framing.
///
/// # Returns
///
/// The number of bytes written to in the `dest` buffer.
///
/// # Panics
///
/// This function will panic if the `dest` buffer is not large enough for the
/// encoded message. You can calculate the size the `dest` buffer needs to be with
/// the [crate::max_encoding_length] function.
pub fn encode(source: &[u8], dest: &mut [u8]) -> usize {
    let mut enc = CobsEncoder::new(dest);
    enc.push(source).unwrap();
    enc.finalize()
}

/// Encodes the `source` buffer into the `dest` buffer, including the default sentinel values 0
/// around the encoded frame.
///
/// # Returns
///
/// The number of bytes written to in the `dest` buffer.
///
/// # Panics
///
/// This function will panic if the `dest` buffer is not large enough for the
/// encoded message. You can calculate the size the `dest` buffer needs to be by adding
/// the [crate::max_encoding_length] function output and 2.
pub fn encode_including_sentinels(source: &[u8], dest: &mut [u8]) -> usize {
    if dest.len() < 2 {
        panic!("destination buffer too small");
    }

    dest[0] = 0;
    let mut enc = CobsEncoder::new(&mut dest[1..]);
    enc.push(source).unwrap();
    let encoded_len = enc.finalize();
    dest[encoded_len + 1] = 0;
    encoded_len + 2
}

/// Attempts to encode the `source` buffer into the `dest` buffer.
///
/// This function assumes the typical sentinel value of 0, but does not terminate the encoded
/// message with the sentinel value. This should be done by the caller to ensure proper framing.
///
/// # Returns
///
/// The number of bytes written to in the `dest` buffer.
///
/// If the destination buffer does not have enough room, an error will be returned.
pub fn try_encode(source: &[u8], dest: &mut [u8]) -> Result<usize, DestBufTooSmallError> {
    let mut enc = CobsEncoder::new(dest);
    enc.push(source)?;
    Ok(enc.finalize())
}

/// Encodes the `source` buffer into the `dest` buffer, including the default sentinel values 0
/// around the encoded frame.
///
/// # Returns
///
/// The number of bytes written to in the `dest` buffer.
///
/// If the destination buffer does not have enough room, an error will be returned.
pub fn try_encode_including_sentinels(
    source: &[u8],
    dest: &mut [u8],
) -> Result<usize, DestBufTooSmallError> {
    if dest.len() < 2 {
        return Err(DestBufTooSmallError);
    }
    dest[0] = 0;
    let mut enc = CobsEncoder::new(&mut dest[1..]);
    enc.push(source)?;
    let encoded_len = enc.finalize();
    *dest.get_mut(encoded_len + 1).ok_or(DestBufTooSmallError)? = 0;
    Ok(encoded_len + 2)
}

/// Encodes the `source` buffer into the `dest` buffer using an
/// arbitrary sentinel value.
///
/// This is done by first encoding the message with the typical sentinel value
/// of 0, then XOR-ing each byte of the encoded message with the chosen sentinel
/// value. This will ensure that the sentinel value doesn't show up in the encoded
/// message. See the paper "Consistent Overhead Byte Stuffing" for details.
///
/// This function does not terminate the encoded message with the sentinel value. This should be
/// done by the caller to ensure proper framing.
///
/// # Returns
///
/// The number of bytes written to in the `dest` buffer.
pub fn encode_with_sentinel(source: &[u8], dest: &mut [u8], sentinel: u8) -> usize {
    let encoded_size = encode(source, dest);
    for x in &mut dest[..encoded_size] {
        *x ^= sentinel;
    }
    encoded_size
}

#[cfg(feature = "alloc")]
/// Encodes the `source` buffer into a vector, using the [encode] function.
pub fn encode_vec(source: &[u8]) -> alloc::vec::Vec<u8> {
    let mut encoded = alloc::vec![0; crate::max_encoding_length(source.len())];
    let encoded_len = encode(source, &mut encoded[..]);
    encoded.truncate(encoded_len);
    encoded
}

#[cfg(feature = "alloc")]
/// Encodes the `source` buffer into a vector, using the [encode] function, while also adding
/// the sentinels around the encoded frame.
pub fn encode_vec_including_sentinels(source: &[u8]) -> alloc::vec::Vec<u8> {
    let mut encoded = alloc::vec![0; crate::max_encoding_length(source.len()) + 2];
    let encoded_len = encode_including_sentinels(source, &mut encoded);
    encoded.truncate(encoded_len);
    encoded
}

#[cfg(feature = "alloc")]
/// Encodes the `source` buffer into a vector with an arbitrary sentinel value, using the
/// [encode_with_sentinel] function.
pub fn encode_vec_with_sentinel(source: &[u8], sentinel: u8) -> alloc::vec::Vec<u8> {
    let mut encoded = alloc::vec![0; crate::max_encoding_length(source.len())];
    let encoded_len = encode_with_sentinel(source, &mut encoded[..], sentinel);
    encoded.truncate(encoded_len);
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "alloc")]
    use crate::{
        decode_vec,
        tests::{test_decode_in_place, test_encode_decode_free_functions},
    };

    #[test]
    fn test_encode_0() {
        // An empty input is encoded as no characters.
        let mut output = [0xFFu8; 16];
        let used = encode(&[], &mut output);
        assert_eq!(used, 1);
        assert_eq!(output[0], 0x01);
    }

    #[cfg(feature = "alloc")]
    fn test_pair(source: &[u8], encoded: &[u8]) {
        test_encode_decode_free_functions(source, encoded);
        test_decode_in_place(source, encoded);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_encode_1() {
        test_pair(&[10, 11, 0, 12], &[3, 10, 11, 2, 12])
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_encode_empty() {
        test_pair(&[], &[1])
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_encode_2() {
        test_pair(&[0, 0, 1, 0], &[1, 1, 2, 1, 1])
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_encode_3() {
        test_pair(&[255, 0], &[2, 255, 1])
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_encode_4() {
        test_pair(&[1], &[2, 1])
    }

    #[test]
    fn encode_target_buf_too_small() {
        let source = &[10, 11, 0, 12];
        let expected = &[3, 10, 11, 2, 12];
        for len in 0..expected.len() {
            let mut dest = [0u8; 5];
            matches!(
                try_encode(source, &mut dest[..len]).unwrap_err(),
                DestBufTooSmallError
            );
        }
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn try_encode_with_sentinels() {
        let source = &[10, 11, 0, 12];
        let expected = &[0, 3, 10, 11, 2, 12, 0];
        let mut dest = alloc::vec![0; expected.len()];
        let encoded_len = try_encode_including_sentinels(source, &mut dest).unwrap();
        assert_eq!(encoded_len, expected.len());
        assert_eq!(dest[0], 0);
        assert_eq!(dest[expected.len() - 1], 0);
        assert_eq!(decode_vec(&dest).unwrap(), source);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_encoding_including_sentinels() {
        let data = [1, 2, 3];
        let encoded = encode_vec_including_sentinels(&data);
        assert_eq!(*encoded.first().unwrap(), 0);
        assert_eq!(*encoded.last().unwrap(), 0);
        let data_decoded = decode_vec(&encoded).unwrap();
        assert_eq!(data_decoded, data);
        let data_decoded = decode_vec(&encoded[1..]).unwrap();
        assert_eq!(data_decoded, data);
        let data_decoded = decode_vec(&encoded[1..encoded.len() - 1]).unwrap();
        assert_eq!(data_decoded, data);
    }

    #[test]
    fn encode_trailing_zero_one_byte_short() {
        // `[1, 0]` encodes to `[2, 1, 1]`; the final code byte must fit too.
        let mut dest = [0xEEu8; 2];
        assert_eq!(try_encode(&[1, 0], &mut dest), Err(DestBufTooSmallError));
        let mut dest = [0xEEu8; 3];
        assert_eq!(try_encode(&[1, 0], &mut dest), Ok(3));
        assert_eq!(dest, [2, 1, 1]);
    }

    #[test]
    #[should_panic]
    fn encode_trailing_zero_one_byte_short_panicking() {
        encode(&[1, 0], &mut [0u8; 2]);
    }

    #[test]
    fn encode_full_run_no_write_past_len() {
        // 254 non-zero bytes encode to `[0xFF, <254 bytes>]`, ensure subsequent
        // bytes are not modified
        for len in [254, 508] {
            let source = [0x11u8; 508];
            let mut dest = [0xEEu8; 520];
            let used = try_encode(&source[..len], &mut dest).unwrap();
            assert_eq!(used, len + len / 254);
            assert!(dest[used..].iter().all(|b| *b == 0xEE), "len={len}");
        }
    }

    #[test]
    fn try_encode_with_sentinels_one_byte_short() {
        // `[1]` encodes to `[0, 2, 1, 0]` with sentinels; no room for the last 0.
        let mut dest = [0xEEu8; 3];
        assert_eq!(
            try_encode_including_sentinels(&[1], &mut dest),
            Err(DestBufTooSmallError)
        );
        let mut dest = [0xEEu8; 4];
        assert_eq!(try_encode_including_sentinels(&[1], &mut dest), Ok(4));
        assert_eq!(dest, [0, 2, 1, 0]);
    }

    #[test]
    #[should_panic]
    fn encode_target_buf_too_small_panicking() {
        let source = &[10, 11, 0, 12];
        let expected = &[3, 10, 11, 2, 12];
        encode(source, &mut [0u8; 5][..expected.len() - 1]);
    }

    #[test]
    fn encode_empty_into_empty_dest() {
        // An empty message encodes to `[0x01]`, which doesn't fit in zero bytes.
        assert_eq!(try_encode(&[], &mut []), Err(DestBufTooSmallError));
        let mut dest = [0xEEu8; 1];
        assert_eq!(try_encode(&[], &mut dest), Ok(1));
        assert_eq!(dest, [0x01]);
    }

    #[test]
    #[should_panic]
    fn encode_empty_into_empty_dest_panicking() {
        encode(&[], &mut []);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn encode_vec_including_sentinels_exact_len() {
        // This input encodes to less than `max_encoding_length`, so the buffer must be
        // truncated right after the closing sentinel.
        let mut source = [0x11u8; 255];
        source[0] = 0;
        let encoded = encode_vec_including_sentinels(&source);
        assert_eq!(encoded.len(), encode_vec(&source).len() + 2);
        assert_eq!(encoded[1..encoded.len() - 1], encode_vec(&source));
    }

    #[test]
    fn empty_push_is_fine() {
        let mut dest = [0xEEu8; 255];
        let source = [0x11; 254];

        let mut enc = CobsEncoder::new(&mut dest);
        assert_eq!(enc.push(&source), Ok(()));
        assert_eq!(enc.finalize(), 255);

        // The presence of an empty push should not trigger us to process a
        // deferred "add a placeholder" case, we wait until the first real data
        // is pushed to do that.
        let mut enc = CobsEncoder::new(&mut dest);
        assert_eq!(enc.push(&source), Ok(()));
        assert_eq!(enc.push(&[]), Ok(()));
        assert_eq!(enc.finalize(), 255);
    }

    #[test]
    fn empty_push_keeps_deferred_code_byte() {
        // After a full run, the next run's code byte is still counted when the first
        // real data arrives, even with empty pushes in between.
        let source = [0x11; 255];
        let mut expected = [0x11; 257];
        expected[0] = 0xFF;
        expected[255] = 0x02;

        let mut dest = [0xEEu8; 257];
        let mut enc = CobsEncoder::new(&mut dest);
        assert_eq!(enc.push(&source[..254]), Ok(()));
        assert_eq!(enc.push(&[]), Ok(()));
        assert_eq!(enc.push(&[]), Ok(()));
        assert_eq!(enc.push(&source[254..]), Ok(()));
        assert_eq!(enc.finalize(), 257);
        assert_eq!(dest, expected);

        let mut decoded = [0u8; 257];
        let n = crate::decode(&dest, &mut decoded).unwrap().frame_size();
        assert_eq!(decoded[..n], source);
    }

    #[test]
    fn push_after_err_fails() {
        // Once a push fails, every later push fails too, even an empty one.
        let mut dest = [0u8; 4];
        let mut enc = CobsEncoder::new(&mut dest);
        assert_eq!(enc.push(&[1, 2, 3, 4, 5]), Err(DestBufTooSmallError));
        assert_eq!(enc.push(&[]), Err(DestBufTooSmallError));
        assert_eq!(enc.push(&[0]), Err(DestBufTooSmallError));
        assert_eq!(enc.finalize(), 0);

        // The same goes for an empty destination.
        let mut enc = CobsEncoder::new(&mut []);
        assert_eq!(enc.push(&[]), Err(DestBufTooSmallError));
        assert_eq!(enc.finalize(), 0);
    }

    #[test]
    fn empty_dest_finalizes_zero() {
        let enc = CobsEncoder::new(&mut []);
        assert_eq!(enc.finalize(), 0);
    }

    #[test]
    fn err_push_finalizes_zero() {
        let mut dest = [0u8; 4];
        let mut enc = CobsEncoder::new(&mut dest);
        assert_eq!(enc.push(&[1, 2, 3, 4, 5]), Err(DestBufTooSmallError));
        assert_eq!(enc.finalize(), 0);
    }
}
