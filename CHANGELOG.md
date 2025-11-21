Change Log
=======

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

# [unreleased]

## Changed

- Bumped MSRV to Rust 1.87 to allow `heapless` update.

# [v0.5.0] 2025-10-21

- Introduce MSRV, which is Rust 1.86
- Move to Rust edition 2024

## Added

- New API to encoded data including the sentinel bytes:
  - `encode_including_sentinels`
  - `try_encode_including_sentinels`
  - `encode_vec_including_sentinels`
- `CobsDecoderOwned` which owns the decoding buffer and does not require lifetimes.
- `CobsDecoderHeapless` which owns the heapless decoding buffer and does not require lifetimes.

# [v0.4.0] 2025-06-21

## Fixed

- The encoder functions will now encode empty input into a single 0x01 byte.
  [#49](https://github.com/jamesmunns/cobs.rs/pull/49)

## Changed

- Bumped `defmt` dependency to v1
  [#51](https://github.com/jamesmunns/cobs.rs/pull/51)
- `decode` now returns the `DecodeReport` structure which contains both the frame size
  and the parsed size.
  [#50](https://github.com/jamesmunns/cobs.rs/pull/50)
- `CobsDecoder::push` also returns a `DecodeReport` now.
  [#50](https://github.com/jamesmunns/cobs.rs/pull/50)
- The `DecodeReport` structure now has `frame_size` and `parsed_size` getter methods instead
  of public `src_used` / `dst_used` fields.
  [#50](https://github.com/jamesmunns/cobs.rs/pull/50)
- The `CobsDecoder` is now able to continuously parse data without having to re-create
  it after each parsed packet.
  [#50](https://github.com/jamesmunns/cobs.rs/pull/50)

# [v0.3.0] 2025-01-31

## Added

- Added `max_encoding_overhead` const function.
  [#20](https://github.com/jamesmunns/cobs.rs/pull/20)
- Unittests for `decode_in_place` and `decode_in_place_report`.
  [#36](https://github.com/jamesmunns/cobs.rs/pull/36)
- New `alloc` feature which enables support for APIs returning `Vec`s.
  [#24](https://github.com/jamesmunns/cobs.rs/pull/24)
- New `std` feature, deprecate the `use_std` feature but keep it for backwards compatiblity.
  [#38](https://github.com/jamesmunns/cobs.rs/pull/38)
- Documentation of the new `std` and `alloc` features in the README.
  [#38](https://github.com/jamesmunns/cobs.rs/pull/38)
- Optional `defmt` support feature-gated by the optional `defmt` feature.
  [#40](https://github.com/jamesmunns/cobs.rs/pull/40)
- Add optional `serde` support that adds `serde` derives for some errors and data structures.
  [#41](https://github.com/jamesmunns/cobs.rs/pull/41)

## Changed

- Bumped rust edition to 2021
- Improved error handling by replacing unit errors with a new `DecodeError` enumeration for
  the decoding module and a `DestBufTooSmallError` for the encoding module.
  [#30](https://github.com/jamesmunns/cobs.rs/pull/30)
- `max_encoding_length` is now a const function. [#20](https://github.com/jamesmunns/cobs.rs/pull/20)
- `decode_in_place` and `decode_in_place_report` now check the destination index and return
  an appropriate `DecodeError` instead of panicking if an out-of-bounds access happens.
  [#36](https://github.com/jamesmunns/cobs.rs/pull/36)
- Consistent behaviour of decode functions for empty input: All funtions now return
  `DecodeError::FrameEmpty`.
  [#39](https://github.com/jamesmunns/cobs.rs/pull/39)

## Fixed

- Fixed wrong encoded length when source length was divisible by 254.
  [#19](https://github.com/jamesmunns/cobs.rs/issues/19)

[unreleased]: https://github.com/jamesmunns/cobs.rs/compare/v0.5.0...HEAD
[v0.5.0]: https://github.com/jamesmunns/cobs.rs/compare/v0.4.0...v0.5.0
[v0.4.0]: https://github.com/jamesmunns/cobs.rs/compare/v0.3.0...v0.4.0
