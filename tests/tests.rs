use cobs::*;

fn test_pair(source: &[u8], encoded: &[u8]) {
    test_encode_decode_free_functions(source, encoded);
    test_decode_in_place(source, encoded);
}

fn test_encode_decode_free_functions(source: &[u8], encoded: &[u8]) {
    let mut test_encoded = encoded.to_vec();
    let mut test_decoded = source.to_vec();

    // Mangle data to ensure data is re-populated correctly
    test_encoded.iter_mut().for_each(|i| *i = 0x80);
    encode(source, &mut test_encoded[..]);
    test_encoded.push(0x00);

    // Mangle data to ensure data is re-populated correctly
    test_decoded.iter_mut().for_each(|i| *i = 0x80);
    decode(&test_encoded, &mut test_decoded[..]).unwrap();
    assert_eq!(encoded, &test_encoded[..test_encoded.len() - 1]);
    assert_eq!(source, test_decoded);
}

fn test_decode_in_place(source: &[u8], encoded: &[u8]) {
    let mut test_encoded = encoded.to_vec();
    let report = decode_in_place_report(&mut test_encoded).unwrap();
    assert_eq!(&test_encoded[0..report.frame_size()], source);
    assert_eq!(report.parsed_size(), encoded.len());

    test_encoded = encoded.to_vec();
    let result = decode_in_place(&mut test_encoded).unwrap();
    assert_eq!(&test_encoded[0..result], source);
}

#[test]
fn stream_continously() {
    let mut dest: [u8; 16] = [0; 16];
    let data = b"hello world";
    let mut encoded_data: [u8; 16] = [0; 16];
    let mut encoded_len = encode(data, &mut encoded_data);
    // Sentinel byte at end.
    encoded_data[encoded_len] = 0x00;
    encoded_len += 1;
    // Stream continously using only `push`. The decoding buffer should not overflow.
    let mut decoder = CobsDecoder::new(&mut dest);
    continuous_decoding(&mut decoder, data, &encoded_data[0..encoded_len]);
}

#[test]
fn stream_continously_2() {
    let mut dest: [u8; 16] = [0; 16];
    let data = b"hello world";
    let mut encoded_data: [u8; 16] = [0; 16];
    let mut encoded_len = encode(data, &mut encoded_data[1..]);
    // Sentinel byte at start and end.
    encoded_data[0] = 0x00;
    encoded_data[encoded_len + 1] = 0x00;
    encoded_len += 2;
    // Stream continously using only `push`. The decoding buffer should not overflow.
    let mut decoder = CobsDecoder::new(&mut dest);
    continuous_decoding(&mut decoder, data, &encoded_data[0..encoded_len]);
}

#[test]
fn stream_chunk_with_2_frames() {
    let mut dest: [u8; 32] = [0; 32];
    let data = b"hello world";
    let data2 = b"second";
    let mut encoded_data: [u8; 32] = [0; 32];
    // Sentinel byte at start.
    encoded_data[0] = 0x00;
    let mut encoded_len = 1;
    encoded_len += encode(data, &mut encoded_data[encoded_len..]);
    // Sentinel byte at end.
    encoded_data[encoded_len] = 0x00;
    encoded_len += 1;
    let first_frame_len = encoded_len;
    encoded_data[encoded_len] = 0x00;
    encoded_len += 1;
    encoded_len += encode(data2, &mut encoded_data[encoded_len..]);
    encoded_data[encoded_len] = 0x00;
    encoded_len += 1;
    let second_frame_len = encoded_len - first_frame_len;
    let mut decoder = CobsDecoder::new(&mut dest);
    let result = decoder.push(&encoded_data[0..encoded_len]).unwrap();
    assert!(result.is_some());
    let frame_report = result.unwrap();
    assert_eq!(frame_report.frame_size(), data.len());
    assert_eq!(frame_report.parsed_size(), first_frame_len);
    // Now insert the rest of the data.
    let result = decoder
        .push(&encoded_data[frame_report.parsed_size()..])
        .unwrap();
    assert!(result.is_some());
    let frame_report = result.unwrap();
    assert_eq!(frame_report.frame_size(), data2.len());
    assert_eq!(frame_report.parsed_size(), second_frame_len);
}

#[test]
fn decoding_broken_packet() {
    let mut dest: [u8; 32] = [0; 32];
    let data = b"hello world";
    let mut encoded_data: [u8; 32] = [0; 32];
    encode(data, &mut encoded_data[1..]);
    let mut encoded_len = encode(data, &mut encoded_data[6..]);
    // Sentinel byte at start and end.
    encoded_data[0] = 0x00;
    // Another frame abruptly starts. This simulates a broken frame, and the streaming decoder
    // should be able to recover from this.
    encoded_data[5] = 0x00;
    encoded_data[5 + encoded_len + 1] = 0x00;
    encoded_len = 5 + encoded_len + 2;
    let mut decoder = CobsDecoder::new(&mut dest);
    for (idx, byte) in encoded_data.iter().take(encoded_len - 1).enumerate() {
        if idx == 5 {
            if let Err(DecodeError::InvalidFrame { decoded_bytes }) = decoder.push(&[*byte]) {
                assert_eq!(decoded_bytes, 3);
            }
        } else {
            decoder.push(&[*byte]).unwrap();
        }
    }
    if let Ok(Some(msg_size)) = decoder.feed(encoded_data[encoded_len - 1]) {
        assert_eq!(msg_size, data.len());
        assert_eq!(data, &decoder.dest()[0..msg_size]);
    } else {
        panic!("decoding call did not yield expected frame");
    }
}

fn continuous_decoding(decoder: &mut CobsDecoder, expected_data: &[u8], encoded_frame: &[u8]) {
    for _ in 0..10 {
        for byte in encoded_frame.iter().take(encoded_frame.len() - 1) {
            decoder.feed(*byte).unwrap();
        }
        if let Ok(Some(sz_msg)) = decoder.feed(encoded_frame[encoded_frame.len() - 1]) {
            assert_eq!(sz_msg, expected_data.len());
            assert_eq!(expected_data, &decoder.dest()[0..sz_msg]);
        } else {
            panic!("decoding call did not yield expected frame");
        }
    }
}

#[test]
fn stream_roundtrip() {
    for ct in 1..=1000 {
        let source: Vec<u8> = (ct..2 * ct).map(|x: usize| (x & 0xFF) as u8).collect();

        let mut dest = vec![0u8; max_encoding_length(source.len())];

        let encoded_size = {
            let mut encoder = CobsEncoder::new(&mut dest);

            for chunk in source.chunks(17) {
                encoder.push(chunk).unwrap();
            }
            encoder.finalize()
        };

        let mut decoded = source.clone();
        decoded.iter_mut().for_each(|i| *i = 0x80);
        let decoded_size = {
            let mut decoder = CobsDecoder::new(&mut decoded);

            for chunk in dest[0..encoded_size].chunks(11) {
                decoder.push(chunk).unwrap();
            }

            match decoder.feed(0) {
                Ok(sz_msg) => sz_msg.unwrap(),
                Err(written) => panic!("decoding failed, {} bytes written to output", written),
            }
        };

        assert_eq!(decoded_size, source.len());
        assert_eq!(source, decoded);
    }
}

#[test]
fn test_max_encoding_length() {
    assert_eq!(max_encoding_length(0), 1);
    assert_eq!(max_encoding_length(253), 254);
    assert_eq!(max_encoding_length(254), 255);
    assert_eq!(max_encoding_length(255), 257);
    assert_eq!(max_encoding_length(254 * 2), 255 * 2);
    assert_eq!(max_encoding_length(254 * 2 + 1), 256 * 2);
}

#[test]
fn test_encode_0() {
    // An empty input is encoded as no characters.
    let mut output = [0xFFu8; 16];
    let used = encode(&[], &mut output);
    assert_eq!(used, 1);
    assert_eq!(output[0], 0x01);
}

#[test]
fn test_encode_1() {
    test_pair(&[10, 11, 0, 12], &[3, 10, 11, 2, 12])
}

#[test]
fn test_encode_empty() {
    test_pair(&[], &[1])
}

#[test]
fn test_encode_2() {
    test_pair(&[0, 0, 1, 0], &[1, 1, 2, 1, 1])
}

#[test]
fn test_encode_3() {
    test_pair(&[255, 0], &[2, 255, 1])
}

#[test]
fn test_encode_4() {
    test_pair(&[1], &[2, 1])
}

#[test]
fn wikipedia_ex_6() {
    let mut unencoded: Vec<u8> = vec![];

    (1..=0xFE).for_each(|i| unencoded.push(i));

    // NOTE: trailing 0x00 is implicit
    let mut encoded: Vec<u8> = vec![];
    encoded.push(0xFF);
    (1..=0xFE).for_each(|i| encoded.push(i));

    test_pair(&unencoded, &encoded);
}

#[test]
fn wikipedia_ex_7() {
    let mut unencoded: Vec<u8> = vec![];

    (0..=0xFE).for_each(|i| unencoded.push(i));

    // NOTE: trailing 0x00 is implicit
    let mut encoded: Vec<u8> = vec![];
    encoded.push(0x01);
    encoded.push(0xFF);
    (1..=0xFE).for_each(|i| encoded.push(i));

    test_pair(&unencoded, &encoded);
}

#[test]
fn wikipedia_ex_8() {
    let mut unencoded: Vec<u8> = vec![];

    (1..=0xFF).for_each(|i| unencoded.push(i));

    // NOTE: trailing 0x00 is implicit
    let mut encoded: Vec<u8> = vec![];
    encoded.push(0xFF);
    (1..=0xFE).for_each(|i| encoded.push(i));
    encoded.push(0x02);
    encoded.push(0xFF);

    test_pair(&unencoded, &encoded);
}

#[test]
fn wikipedia_ex_9() {
    let mut unencoded: Vec<u8> = vec![];

    (2..=0xFF).for_each(|i| unencoded.push(i));
    unencoded.push(0x00);

    // NOTE: trailing 0x00 is implicit
    let mut encoded: Vec<u8> = vec![];
    encoded.push(0xFF);
    (2..=0xFF).for_each(|i| encoded.push(i));
    encoded.push(0x01);
    encoded.push(0x01);

    test_pair(&unencoded, &encoded);
}

#[test]
fn wikipedia_ex_10() {
    let mut unencoded: Vec<u8> = vec![];

    (3..=0xFF).for_each(|i| unencoded.push(i));
    unencoded.push(0x00);
    unencoded.push(0x01);

    // NOTE: trailing 0x00 is implicit
    let mut encoded: Vec<u8> = vec![];
    encoded.push(0xFE);
    (3..=0xFF).for_each(|i| encoded.push(i));
    encoded.push(0x02);
    encoded.push(0x01);

    test_pair(&unencoded, &encoded);
}

#[test]
fn issue_15() {
    // Reported: https://github.com/awelkie/cobs.rs/issues/15

    let my_string_buf = b"\x00\x11\x00\x22";
    let max_len = max_encoding_length(my_string_buf.len());
    assert!(max_len < 128);
    let mut buf = [0u8; 128];

    let len = encode_with_sentinel(my_string_buf, &mut buf, b'\x00');

    let cobs_buf = &buf[0..len];

    let mut decoded_dest_buf = [0u8; 128];
    let new_len = decode_with_sentinel(cobs_buf, &mut decoded_dest_buf, b'\x00').unwrap();
    let decoded_buf = &decoded_dest_buf[0..new_len];

    println!("{:?}  {:?}  {:?}", my_string_buf, cobs_buf, decoded_buf);
    assert_eq!(my_string_buf, decoded_buf);
}

#[test]
fn issue_19_test_254_block_all_ones() {
    let src: [u8; 254] = [1; 254];
    let mut dest: [u8; 256] = [0; 256];
    let encode_len = encode(&src, &mut dest);
    assert_eq!(encode_len, 255);
    let mut decoded: [u8; 254] = [1; 254];
    let result = decode(&dest, &mut decoded).expect("decoding failed");
    assert_eq!(result.frame_size(), 254);
    assert_eq!(result.parsed_size(), 256);
    assert_eq!(&src, &decoded);
}

#[cfg(feature = "alloc")]
mod alloc_tests {
    use super::*;
    use quickcheck::{quickcheck, TestResult};

    #[test]
    fn test_roundtrip_1() {
        test_roundtrip(&[1, 2, 3]);
    }

    #[test]
    fn test_roundtrip_2() {
        for i in 0..5usize {
            let mut v = Vec::new();
            for j in 0..252 + i {
                v.push(j as u8);
            }
            test_roundtrip(&v);
        }
    }

    fn identity(source: Vec<u8>, sentinel: u8) -> TestResult {
        let encoded = encode_vec_with_sentinel(&source[..], sentinel);

        if source.is_empty() {
            return TestResult::passed();
        }

        // Check that the sentinel doesn't show up in the encoded message
        for x in encoded.iter() {
            if *x == sentinel {
                return TestResult::error("Sentinel found in encoded message.");
            }
        }

        // Check that the decoding the encoded message returns the original message
        match decode_vec_with_sentinel(&encoded[..], sentinel) {
            Ok(decoded) => {
                if source == decoded {
                    TestResult::passed()
                } else {
                    TestResult::failed()
                }
            }
            Err(_) => TestResult::error("decoding Error"),
        }
    }

    #[test]
    fn test_encode_decode_with_sentinel() {
        quickcheck(identity as fn(Vec<u8>, u8) -> TestResult);
    }

    #[test]
    fn test_encode_decode() {
        fn identity_default_sentinel(source: Vec<u8>) -> TestResult {
            identity(source, 0)
        }
        quickcheck(identity_default_sentinel as fn(Vec<u8>) -> TestResult);
    }

    fn test_roundtrip(source: &[u8]) {
        let mut encoded = encode_vec(source);
        // Terminate the frame.
        encoded.push(0x00);
        let decoded = decode_vec(&encoded).expect("decode_vec");
        assert_eq!(source, decoded);
    }
}
