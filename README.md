# COBS.rs

[![Build Status](https://travis-ci.org/awelkie/cobs.rs.svg?branch=master)](https://travis-ci.org/awelkie/cobs.rs)
[![](https://img.shields.io/crates/v/cobs.svg)](https://crates.io/crates/cobs)
[![](https://img.shields.io/crates/l/cobs.svg)](https://crates.io/crates/cobs)

This is an implementation of the Consistent Overhead Byte Stuffing (COBS) algorithm in Rust.

COBS is an algorithm for transforming a message into an encoding where a specific value (the "sentinel" value) is not used. This value can then be used to mark frame boundaries in a serial communication channel.

See www.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing for details.
