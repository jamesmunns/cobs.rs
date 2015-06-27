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
    encode_with_sentinel(source, dest, 0)
}

/// Encodes the `source` buffer into the `dest` buffer using an
/// arbitrary sentinel value.
pub fn encode_with_sentinel(source: &[u8], dest: &mut[u8], sentinel: u8) -> usize {
    let mut dest_index = 1;
    let mut code_index = 0;
    let mut num_between_sentinel = 1;

    if source.is_empty() {
        return 0;
    }

    for x in source {
        if *x == sentinel {
            dest[code_index] = num_between_sentinel;
            num_between_sentinel = 1;
            code_index = dest_index;
            dest_index += 1;
        } else {
            dest[dest_index] = *x;
            num_between_sentinel += 1;
            dest_index += 1;
            if 0xFF == num_between_sentinel {
                dest[code_index] = num_between_sentinel;
                num_between_sentinel = 1;
                code_index = dest_index;
                dest_index += 1;
            }
        }
    }

    dest[code_index] = num_between_sentinel;

    return dest_index;
}

/// Encodes the `source` buffer into a vector.
pub fn encode_vec(source: &[u8]) -> Vec<u8> {
    encode_vec_with_sentinel(source, 0)
}

/// Encodes the `source` buffer into a vector with an arbitrary sentinel value.
pub fn encode_vec_with_sentinel(source: &[u8], sentinel: u8) -> Vec<u8> {
    let mut encoded = vec![0; max_encoding_length(source.len())];
    let encoded_len = encode_with_sentinel(source, &mut encoded[..], sentinel);
    encoded.truncate(encoded_len);
    return encoded;
}

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
    decode_with_sentinel(source, dest, 0)
}

/// Decodes the `source` buffer into the `dest` buffer using an arbitrary sentinel value.
pub fn decode_with_sentinel(source: &[u8], dest: &mut[u8], sentinel: u8) -> Result<usize, ()> {
    let mut source_index = 0;
    let mut dest_index = 0;

    while source_index < source.len() {
        let code = source[source_index];

        if source_index + code as usize > source.len() && code != 1 {
            return Err(());
        }

        source_index += 1;

        for _ in (1..code) {
            dest[dest_index] = source[source_index];
            source_index += 1;
            dest_index += 1;
        }

        if 0xFF != code && source_index < source.len() {
            dest[dest_index] = sentinel;
            dest_index += 1;
        }
    }

    Ok(dest_index)
}

/// Decodes the `source` buffer into a vector.
pub fn decode_vec(source: &[u8]) -> Result<Vec<u8>, ()> {
    decode_vec_with_sentinel(source, 0)
}

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
