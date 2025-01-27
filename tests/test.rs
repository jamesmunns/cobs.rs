extern crate cobs;
extern crate quickcheck;

use cobs::{decode, decode_vec, encode, encode_vec, max_encoding_length};
use cobs::{decode_vec_with_sentinel, encode_vec_with_sentinel};
use cobs::{CobsDecoder, CobsEncoder};
use quickcheck::{quickcheck, TestResult};

fn test_pair(source: Vec<u8>, encoded: Vec<u8>) {
    let mut test_encoded = encoded.clone();
    let mut test_decoded = source.clone();

    // Mangle data to ensure data is re-populated correctly
    test_encoded.iter_mut().for_each(|i| *i = 0x80);
    encode(&source[..], &mut test_encoded[..]);

    // Mangle data to ensure data is re-populated correctly
    test_decoded.iter_mut().for_each(|i| *i = 0x80);
    decode(&encoded[..], &mut test_decoded[..]).unwrap();

    assert_eq!(encoded, test_encoded);
    assert_eq!(source, test_decoded);
}

fn test_roundtrip(source: Vec<u8>) {
    let encoded = encode_vec(&source);
    let decoded = decode_vec(&encoded).expect("decode_vec");
    assert_eq!(source, decoded);
}

#[test]
fn decode_malforemd() {
    let malformed_buf: [u8; 32] = [
        68, 69, 65, 68, 66, 69, 69, 70, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ];
    let mut dest_buf: [u8; 32] = [0; 32];
    if let Err(()) = decode(&malformed_buf, &mut dest_buf) {
        return;
    } else {
        assert!(false, "invalid test result.");
    }
}

#[test]
fn stream_roundtrip() {
    for ct in 1..=1000 {
        let source: Vec<u8> = (ct..2 * ct).map(|x: usize| (x & 0xFF) as u8).collect();

        let mut dest = vec![0u8; max_encoding_length(source.len())];

        let sz_en = {
            let mut ce = CobsEncoder::new(&mut dest);

            for c in source.chunks(17) {
                ce.push(c).unwrap();
            }
            let sz = ce.finalize().unwrap();
            sz
        };

        let mut decoded = source.clone();
        decoded.iter_mut().for_each(|i| *i = 0x80);
        let sz_de = {
            let mut cd = CobsDecoder::new(&mut decoded);

            for c in dest[0..sz_en].chunks(11) {
                cd.push(c).unwrap();
            }
            let sz_msg = match cd.feed(0) {
                Ok(sz_msg) => sz_msg.unwrap(),
                Err(written) => panic!("decoding failed, {} bytes written to output", written),
            };
            sz_msg
        };

        assert_eq!(sz_de, source.len());
        assert_eq!(source, decoded);
    }
}

#[test]
fn test_max_encoding_length() {
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
    assert_eq!(used, 0);
    assert_eq!(output.as_slice(), &[0xFFu8; 16]);
}

#[test]
fn test_encode_1() {
    test_pair(vec![10, 11, 0, 12], vec![3, 10, 11, 2, 12])
}

#[test]
fn test_encode_2() {
    test_pair(vec![0, 0, 1, 0], vec![1, 1, 2, 1, 1])
}

#[test]
fn test_encode_3() {
    test_pair(vec![255, 0], vec![2, 255, 1])
}

#[test]
fn test_encode_4() {
    test_pair(vec![1], vec![2, 1])
}

#[test]
fn test_roundtrip_1() {
    test_roundtrip(vec![1, 2, 3]);
}

#[test]
fn test_roundtrip_2() {
    for i in 0..5usize {
        let mut v = Vec::new();
        for j in 0..252 + i {
            v.push(j as u8);
        }
        test_roundtrip(v);
    }
}

fn identity(source: Vec<u8>, sentinel: u8) -> TestResult {
    let encoded = encode_vec_with_sentinel(&source[..], sentinel);

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
        Err(_) => TestResult::error("Decoding Error"),
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

#[test]
fn wikipedia_ex_6() {
    let mut unencoded: Vec<u8> = vec![];

    (1..=0xFE).for_each(|i| unencoded.push(i));

    // NOTE: trailing 0x00 is implicit
    let mut encoded: Vec<u8> = vec![];
    encoded.push(0xFF);
    (1..=0xFE).for_each(|i| encoded.push(i));

    test_pair(unencoded, encoded);
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

    test_pair(unencoded, encoded);
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

    test_pair(unencoded, encoded);
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

    test_pair(unencoded, encoded);
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

    test_pair(unencoded, encoded);
}

#[test]
fn issue_15() {
    // Reported: https://github.com/awelkie/cobs.rs/issues/15

    let my_string_buf = b"\x00\x11\x00\x22";
    let max_len = cobs::max_encoding_length(my_string_buf.len());
    assert!(max_len < 128);
    let mut buf = [0u8; 128];

    let len = cobs::encode_with_sentinel(my_string_buf, &mut buf, b'\x00');

    let cobs_buf = &buf[0..len];

    let mut decoded_dest_buf = [0u8; 128];
    let new_len = cobs::decode_with_sentinel(cobs_buf, &mut decoded_dest_buf, b'\x00').unwrap();
    let decoded_buf = &decoded_dest_buf[0..new_len];

    println!("{:?}  {:?}  {:?}", my_string_buf, cobs_buf, decoded_buf);
    assert_eq!(my_string_buf, decoded_buf);
}

#[test]
fn issue_19_test_254_block_all_ones() {
    let src: [u8; 254] = [1; 254];
    let mut dest: [u8; 256] = [0; 256];
    let encode_len = encode(&src, &mut dest);
    //println!("Encoded buf [0..100]: {:x?}", &dest[0..100]);
    //println!("Encoded buf [100..end]: {:x?}", &dest[100..]);
    assert_eq!(encode_len, 255);
    let mut decoded: [u8; 254] = [1; 254];
    let decoded_len = decode(&dest, &mut decoded).expect("decoding failed");
    assert_eq!(decoded_len, 254);
    assert_eq!(&src, &decoded);
}
