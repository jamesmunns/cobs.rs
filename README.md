# `postcard-cobs`

> ⚠️ - WARNING
>
> This crate is a fork of the [cobs.rs] crate, originally intended to
> be a temporary solution for the [postcard] crate. Since then,
> [@jamesmunns] has been granted ownership of the [cobs.rs] crate, and
> intends to upstream the local changes in this crate.
>
> At that time, this crate will be marked `deprecated` with a point
> release, but will NOT be yanked.

[cobs.rs]: https://github.com/awelkie/cobs.rs
[postcard]: https://docs.rs/postcard
[@jamesmunns]: https://github.com/jamesmunns

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
