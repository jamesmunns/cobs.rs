#![cfg_attr(not(feature = "use_std"), no_std)]

#[derive(Debug)]
pub struct CobsEncoder<'a> {
    dest: &'a mut [u8],
    dest_idx: usize,
    state: EncoderState,
}

pub enum PushResult {
    AddSingle(u8),
    ModifyFromStartAndSkip((usize, u8)),
    ModifyFromStartAndPushAndSkip((usize, u8, u8))
}

#[derive(Debug)]
pub struct EncoderState {
    code_idx: usize,
    num_bt_sent: u8,
    offset_idx: u8,
}

impl Default for EncoderState {
    fn default() -> Self {
        Self {
            code_idx: 0,
            num_bt_sent: 1,
            offset_idx: 1,
        }
    }
}

impl EncoderState {
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

#[derive(Debug)]
pub struct CobsDecoder<'a> {
    /// Destination slice for decoded message
    dest: &'a mut [u8],

    /// Index of next byte to write in `dest`
    dest_idx: usize,

    /// Decoder state as an enum
    state: DecoderState,
}

#[derive(Debug)]
pub enum DecoderState {
    /// State machine has not received any non-zero bytes
    Idle,

    /// 1-254 bytes, can be header or 00
    Grab(u8),

    /// 255 bytes, will be a header next
    GrabChain(u8),
}

fn add(to: &mut [u8], idx: usize, data: u8) -> Result<(), ()> {
    *to.get_mut(idx)
        .ok_or_else(|| ())? = data;
    Ok(())
}

pub enum DecodeResult {
    NoData,
    DataComplete,
    DataContinue(u8),
}

impl DecoderState {
    pub fn feed(&mut self, data: u8) -> Result<DecodeResult, ()> {
        use DecoderState::*;
        use DecodeResult::*;
        let (ret, state) = match (&self, data) {
            // Currently Idle, received a terminator, ignore, stay idle
            (Idle, 0x00) => (Ok(NoData), Idle),

            // Currently Idle, received a byte indicating the
            // next 255 bytes have no zeroes, so we will have 254 unmodified
            // data bytes, then an overhead byte
            (Idle, 0xFF) => (Ok(NoData), GrabChain(0xFE)),

            // Currently Idle, received a byte indicating there will be a
            // zero that must be modified in the next 1..=254 bytes
            (Idle, n)    => (Ok(NoData), Grab(n - 1)),

            // We have reached the end of a data run indicated by an overhead
            // byte, AND we have recieved the message terminator. This was a
            // well framed message!
            (Grab(0), 0x00) => (Ok(DataComplete), Idle),

            // We have reached the end of a data run indicated by an overhead
            // byte, and the next segment of 254 bytes will have no modified
            // sentinel bytes
            (Grab(0), 0xFF) => {
                (Ok(DataContinue(0)), GrabChain(0xFE))
            },

            // We have reached the end of a data run indicated by an overhead
            // byte, and we will treat this byte as a modified sentinel byte.
            // place the sentinel byte in the output, and begin processing the
            // next non-sentinel sequence
            (Grab(0), n) => {
                (Ok(DataContinue(0)), Grab(n - 1))
            },

            // We were not expecting the sequence to terminate, but here we are.
            // Report an error due to early terminated message
            (Grab(_), 0) => {
                (Err(()), Idle)
            }

            // We have not yet reached the end of a data run, decrement the run
            // counter, and place the byte into the decoded output
            (Grab(i), n) =>  {
                (Ok(DataContinue(n)), Grab(*i - 1))
            },

            // We have reached the end of a data run indicated by an overhead
            // byte, AND we have recieved the message terminator. This was a
            // well framed message!
            (GrabChain(0), 0x00) => {
                (Ok(DataComplete), Idle)
            }

            // We have reached the end of a data run, and we will begin another
            // data run with an overhead byte expected at the end
            (GrabChain(0), 0xFF) => (Ok(NoData), GrabChain(0xFE)),

            // We have reached the end of a data run, and we will expect `n` data
            // bytes unmodified, followed by a sentinel byte that must be modified
            (GrabChain(0), n) => (Ok(NoData), Grab(n - 1)),

            // We were not expecting the sequence to terminate, but here we are.
            // Report an error due to early terminated message
            (GrabChain(_), 0) => {
                (Err(()), Idle)
            }

            // We have not yet reached the end of a data run, decrement the run
            // counter, and place the byte into the decoded output
            (GrabChain(i), n) => {
                (Ok(DataContinue(n)), GrabChain(*i - 1))
            },
        };

        *self = state;
        ret
    }
}

impl<'a> CobsDecoder<'a> {

    /// Create a new streaming Cobs Decoder. Provide the output buffer
    /// for the decoded message to be placed in
    pub fn new(dest: &'a mut [u8]) -> CobsDecoder<'a> {
        CobsDecoder {
            dest,
            dest_idx: 0,
            state: DecoderState::Idle,
        }
    }

    /// Push a single byte into the streaming CobsDecoder. Return values mean:
    ///
    /// * Ok(None) - State machine okay, more data needed
    /// * Ok(Some(N)) - A message of N bytes was successfully decoded
    /// * Err(M) - Message decoding failed, and M bytes were written to output
    ///
    /// NOTE: Sentinel value must be included in the input to this function for the
    /// decoding to complete
    pub fn feed(&mut self, data: u8) -> Result<Option<usize>, usize> {
        match self.state.feed(data) {
            Err(()) => Err(self.dest_idx),
            Ok(DecodeResult::NoData) => Ok(None),
            Ok(DecodeResult::DataContinue(n)) => {
                add(self.dest, self.dest_idx, n).map_err(|_| self.dest_idx)?;
                self.dest_idx += 1;
                Ok(None)
            }
            Ok(DecodeResult::DataComplete) => {
                Ok(Some(self.dest_idx))
            }
        }
    }

    /// Push a slice of bytes into the streaming CobsDecoder. Return values mean:
    ///
    /// * Ok(None) - State machine okay, more data needed
    /// * Ok(Some((N, M))) - A message of N bytes was successfully decoded,
    ///     using M bytes from `data` (and earlier data)
    /// * Err(J) - Message decoding failed, and J bytes were written to output
    ///
    /// NOTE: Sentinel value must be included in the input to this function for the
    /// decoding to complete
    pub fn push(&mut self, data: &[u8]) -> Result<Option<(usize, usize)>, usize> {
        for (consumed_idx, d) in data.iter().enumerate() {
            let x = self.feed(*d);
            if let Some(decoded_bytes_ct) = x? {
                // convert from index to number of bytes consumed
                return Ok(Some((decoded_bytes_ct, consumed_idx + 1)));
            }
        }

        Ok(None)
    }
}

// This needs to be a macro because `src` and `dst` could be the same or different.
macro_rules! decode_raw (
    ($src:ident, $dst:ident) => ({
        let mut source_index = 0;
        let mut dest_index = 0;

        while source_index < $src.len() {
            let code = $src[source_index];

            if source_index + code as usize > $src.len() && code != 1 {
                return Err(());
            }

            source_index += 1;

            for _ in 1..code {
                $dst[dest_index] = $src[source_index];
                source_index += 1;
                dest_index += 1;
            }

            if 0xFF != code && source_index < $src.len() {
                $dst[dest_index] = 0;
                dest_index += 1;
            }
        }

        Ok(dest_index)
    })
);

/// Decodes the `source` buffer into the `dest` buffer.
///
/// This function uses the typical sentinel value of 0.
///
/// # Failures
///
/// This will return `Err(())` if there was a decoding error. Otherwise,
/// it will return `Ok(n)` where `n` is the length of the decoded message.
///
/// # Panics
///
/// This function will panic if the `dest` buffer is not large enough for the
/// decoded message. Since an encoded message as always larger than a decoded
/// message, it may be a good idea to make the `dest` buffer as big as the
/// `source` buffer.
pub fn decode(source: &[u8], dest: &mut[u8]) -> Result<usize, ()> {
    let mut dec = CobsDecoder::new(dest);
    assert!(dec.push(source).unwrap().is_none());

    // Explicitly push sentinel of zero
    if let Some((d_used, _s_used)) = dec.push(&[0]).unwrap() {
        Ok(d_used)
    } else {
        Err(())
    }
}

/// Decodes a message in-place.
///
/// This is the same function as `decode`, but replaces the encoded message
/// with the decoded message instead of writing to another buffer.
pub fn decode_in_place(buff: &mut[u8]) -> Result<usize, ()> {
    decode_raw!(buff, buff)
}

/// Decodes the `source` buffer into the `dest` buffer using an arbitrary sentinel value.
///
/// This is done by XOR-ing each byte of the source message with the chosen sentinel value,
/// which transforms the message into the same message encoded with a sentinel value of 0.
/// Then the regular decoding transformation is performed.
pub fn decode_with_sentinel(source: &[u8], dest: &mut[u8], sentinel: u8) -> Result<usize, ()> {
    for (x, y) in source.iter().zip(dest.iter_mut()) {
        *y = *x ^ sentinel;
    }
    decode_in_place(dest)
}

/// Decodes a message in-place using an arbitrary sentinel value.
pub fn decode_in_place_with_sentinel(buff: &mut[u8], sentinel: u8) -> Result<usize, ()> {
    for x in buff.iter_mut() {
        *x ^= sentinel;
    }
    decode_in_place(buff)
}

#[cfg(feature = "use_std")]
/// Decodes the `source` buffer into a vector.
pub fn decode_vec(source: &[u8]) -> Result<Vec<u8>, ()> {
    let mut decoded = vec![0; source.len()];
    match decode(source, &mut decoded[..]) {
        Ok(n) => {
            decoded.truncate(n);
            Ok(decoded)
        },
        Err(()) => Err(()),
    }
}

#[cfg(feature = "use_std")]
/// Decodes the `source` buffer into a vector with an arbitrary sentinel value.
pub fn decode_vec_with_sentinel(source: &[u8], sentinel: u8) -> Result<Vec<u8>, ()> {
    let mut decoded = vec![0; source.len()];
    match decode_with_sentinel(source, &mut decoded[..], sentinel) {
        Ok(n) => {
            decoded.truncate(n);
            Ok(decoded)
        },
        Err(()) => Err(()),
    }
}

/// Calculates the maximum possible size of an encoded message given the length
/// of the source message. This may be useful for calculating how large the
/// `dest` buffer needs to be in the encoding functions.
pub fn max_encoding_length(source_len: usize) -> usize {
    source_len + (source_len / 254) + if source_len % 254 > 0 { 1 } else { 0 }
}
