#[cfg(feature = "use_std")]
use crate::max_encoding_length;

/// The [`CobsEncoder`] type is used to encode a stream of bytes to a
/// given mutable output slice. This is often useful when heap data
/// structures are not available, or when not all message bytes are
/// received at a single point in time.
#[derive(Debug)]
pub struct CobsEncoder<'a> {
    dest: &'a mut [u8],
    dest_idx: usize,
    state: EncoderState,
}

/// The [`EncoderState`] is used to track the current state of a
/// streaming encoder. This struct does not contain the output buffer
/// (or a reference to one), and can be used when streaming the encoded
/// output to a custom data type
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
    /// the current end of the output buffer to be later modified
    ModifyFromStartAndPushAndSkip((usize, u8, u8))
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
                let ret = PushResult::ModifyFromStartAndPushAndSkip((self.code_idx, self.num_bt_sent, data));
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

impl<'a> CobsEncoder<'a> {

    /// Create a new streaming Cobs Encoder
    pub fn new(out_buf: &'a mut [u8]) -> CobsEncoder<'a> {
        CobsEncoder {
            dest: out_buf,
            dest_idx: 1,
            state: EncoderState::default(),
        }
    }

    /// Push a slice of data to be encoded
    pub fn push(&mut self, data: &[u8]) -> Result<(), ()> {
        // TODO: could probably check if this would fit without
        // iterating through all data
        for x in data {
            use PushResult::*;
            match self.state.push(*x) {
                AddSingle(y) => {
                    *self.dest.get_mut(self.dest_idx)
                        .ok_or_else(|| ())? = y;
                }
                ModifyFromStartAndSkip((idx, mval)) => {
                    *self.dest.get_mut(idx)
                        .ok_or_else(|| ())? = mval;
                }
                ModifyFromStartAndPushAndSkip((idx, mval, nval1)) => {
                    *self.dest.get_mut(idx)
                        .ok_or_else(|| ())? = mval;
                    *self.dest.get_mut(self.dest_idx)
                        .ok_or_else(|| ())? = nval1;
                    self.dest_idx += 1;
                }
            }

            // All branches above require advancing the pointer at least once
            self.dest_idx += 1;
        }

        Ok(())
    }

    /// Complete encoding of the output message. Does NOT terminate
    /// the message with the sentinel value
    pub fn finalize(self) -> Result<usize, ()> {
        if self.dest_idx == 1 {
            return Ok(0);
        }

        // Get the last index that needs to be fixed
        let (idx, mval) = self.state.finalize();

        // If the current code index is outside of the destination slice,
        // we do not need to write it out
        if let Some(i) = self.dest.get_mut(idx) {
            *i = mval;
        }

        return Ok(self.dest_idx);
    }
}

/// Encodes the `source` buffer into the `dest` buffer.
///
/// This function uses the typical sentinel value of 0. It returns the number of bytes
/// written to in the `dest` buffer.
///
/// # Panics
///
/// This function will panic if the `dest` buffer is not large enough for the
/// encoded message. You can calculate the size the `dest` buffer needs to be with
/// the `max_encoding_length` function.
pub fn encode(source: &[u8], dest: &mut[u8]) -> usize {
    let mut enc = CobsEncoder::new(dest);
    enc.push(source).unwrap();
    enc.finalize().unwrap()
}

/// Attempts to encode the `source` buffer into the `dest` buffer.
///
/// This function uses the typical sentinel value of 0. It returns the number of bytes
/// written to in the `dest` buffer.
///
/// If the destination buffer does not have enough room, an error will be returned
pub fn try_encode(source: &[u8], dest: &mut[u8]) -> Result<usize, ()> {
    let mut enc = CobsEncoder::new(dest);
    enc.push(source)?;
    enc.finalize()
}

/// Encodes the `source` buffer into the `dest` buffer using an
/// arbitrary sentinel value.
///
/// This is done by first encoding the message with the typical sentinel value
/// of 0, then XOR-ing each byte of the encoded message with the chosen sentinel
/// value. This will ensure that the sentinel value doesn't show up in the encoded
/// message. See the paper "Consistent Overhead Byte Stuffing" for details.
pub fn encode_with_sentinel(source: &[u8], dest: &mut[u8], sentinel: u8) -> usize {
    let encoded_size = encode(source, dest);
    for x in &mut dest[..encoded_size] {
        *x ^= sentinel;
    }
    return encoded_size;
}

#[cfg(feature = "use_std")]
/// Encodes the `source` buffer into a vector.
pub fn encode_vec(source: &[u8]) -> Vec<u8> {
    let mut encoded = vec![0; max_encoding_length(source.len())];
    let encoded_len = encode(source, &mut encoded[..]);
    encoded.truncate(encoded_len);
    return encoded;
}

#[cfg(feature = "use_std")]
/// Encodes the `source` buffer into a vector with an arbitrary sentinel value.
pub fn encode_vec_with_sentinel(source: &[u8], sentinel: u8) -> Vec<u8> {
    let mut encoded = vec![0; max_encoding_length(source.len())];
    let encoded_len = encode_with_sentinel(source, &mut encoded[..], sentinel);
    encoded.truncate(encoded_len);
    return encoded;
}
