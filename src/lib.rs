#![feature(no_std)]
#[no_std]

pub fn encode(source: &[u8], dest: &mut[u8]) {
    encode_with_sentinel(source, dest, 0)
}

pub fn encode_with_sentinel(source: &[u8], dest: &mut[u8], sentinel: u8) {
    let mut dest_index = 1;
    let mut code_index = 0;
    let mut num_between_sentinel = 1;

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

#[test]
fn test_encode_1() {
    let source = vec![10, 11, 0, 12];
    let encoded = vec![3, 10, 11, 2, 12];
    let mut test_encoded = encoded.clone();
    encode(&source[..], &mut test_encoded[..]);
    assert_eq!(encoded, test_encoded);
}
