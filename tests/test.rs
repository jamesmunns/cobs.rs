extern crate cobs;
extern crate quickcheck;

use quickcheck::{quickcheck, TestResult};
use cobs::{max_encoding_length, encode, decode, encode_vec, decode_vec};
use cobs::{encode_vec_with_sentinel, decode_vec_with_sentinel};

fn test_pair(source: Vec<u8>, encoded: Vec<u8>) {
    let mut test_encoded = encoded.clone();
    let mut test_decoded = source.clone();
    encode(&source[..], &mut test_encoded[..]);
    decode(&encoded[..], &mut test_decoded[..]).unwrap();
    assert_eq!(encoded, test_encoded);
    assert_eq!(source, test_decoded);
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

fn identity(source: Vec<u8>, sentinel: u8) -> TestResult {
    let encoded = encode_vec_with_sentinel(&source[..], sentinel);
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
