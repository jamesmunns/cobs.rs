# `cobs`

This is an implementation of the Consistent Overhead Byte Stuffing (COBS) algorithm in Rust.

COBS is an algorithm for transforming a message into an encoding where a specific value (the "sentinel" value) is not used. This value can then be used to mark frame boundaries in a serial communication channel.

See www.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing for details.

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
