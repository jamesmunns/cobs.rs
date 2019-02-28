extern crate cobs;
extern crate quickcheck;

use quickcheck::{quickcheck, TestResult};
use cobs::{max_encoding_length, encode, decode, encode_vec, decode_vec};
use cobs::{encode_vec_with_sentinel, decode_vec_with_sentinel};
use cobs::{CobsEncoder};

fn test_pair(source: Vec<u8>, encoded: Vec<u8>) {
    let mut test_encoded = encoded.clone();
    let mut test_decoded = source.clone();
    encode(&source[..], &mut test_encoded[..]);
    decode(&encoded[..], &mut test_decoded[..]).unwrap();
    assert_eq!(encoded, test_encoded);
    assert_eq!(source, test_decoded);
}

fn test_roundtrip(source: Vec<u8>) {
    let encoded = encode_vec(&source);
    println!("E: {:?}", encoded);
    let decoded = decode_vec(&encoded).expect("decode_vec");
    println!("D: {:?}", decoded);
    assert_eq!(source, decoded);
}

#[test]
fn cobs_encoder() {
    #[allow(overflowing_literals)]
    let source: Vec<u8> = (0..1200)
        .map(|x: usize| (x & 0xFF) as u8)
        .collect();

    let mut dest = [0u8; 1300];
    let mut dest2 = [0u8; 1300];
    let mut ddest = [0u8; 1300];

    let sz = {
        let mut ce = CobsEncoder::new(&mut dest);

        for c in source.chunks(15) {
            ce.push(c).unwrap();
        }
        let sz = ce.finalize().unwrap();
        sz
    };

    let enc = encode(source.as_slice(), &mut dest2);
    assert_eq!(enc, sz);
    assert_eq!(dest.to_vec(), dest2.to_vec());

    let decoded = decode(&dest[..sz], &mut ddest).unwrap();
    assert_eq!(decoded, source.len());
    assert_eq!(source.as_slice(), &ddest[..decoded]);
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
    test_roundtrip(vec![1,2,3]);
}

#[test]
fn test_roundtrip_2() {
    for i in 0..5usize {
        let mut v = Vec::new();
        for j in 0..252+i {
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
