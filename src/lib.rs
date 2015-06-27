pub fn encode(source: &[u8], dest: &mut[u8]) -> usize {
    encode_with_sentinel(source, dest, 0)
}

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

pub fn encode_vec(source: &[u8]) -> Vec<u8> {
    encode_vec_with_sentinel(source, 0)
}

pub fn encode_vec_with_sentinel(source: &[u8], sentinel: u8) -> Vec<u8> {
    let mut encoded = vec![0; max_encoding_length(source.len())];
    let encoded_len = encode_with_sentinel(source, &mut encoded[..], sentinel);
    encoded.truncate(encoded_len);
    return encoded;
}

pub fn decode(source: &[u8], dest: &mut[u8]) -> Result<usize, ()> {
    decode_with_sentinel(source, dest, 0)
}

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

pub fn decode_vec(source: &[u8]) -> Result<Vec<u8>, ()> {
    decode_vec_with_sentinel(source, 0)
}

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


pub fn max_encoding_length(source_len: usize) -> usize {
    source_len + (source_len / 254) + if source_len % 254 > 0 { 1 } else { 0 }
}
