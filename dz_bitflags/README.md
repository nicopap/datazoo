# DZ Bitflags

Packed structs for compact and user-controlled representation of complex data.

## Alternatives

- [`packed_struct`]: A venerable crate used in many different libraries, allow
  reading structs from packed `&[u8]` and vis-versa
- [`bondrewd`]: An old crate generating "reader" and "writers" so that specific
  fields can be read individually, avoiding the cost of unpacking other fields
- [`bitbybit`]: I'm shocked I didn't find this earlier, it does everything this
  crate does.

## Why DZ Bitflags

- Unlike the alernative, you can pack and unpack from other types than `[u8]`.
  You can explicitly declare the packed type, a compilation error occurs if
  the packed size is larger than the choosen packed backing storage.
  Supported types are `u8, u16, u32, u64` and fixed size arrays thereof.
- It uses the [`arbitrary-int`] crate, which exports types in the style of [`ux`]
  but with much less compilation time overhead. You can use `u6`, `u12` in your
  structs. It is also stable.
- It optionally implements readers in the style of `bondrewd` to avoid unpacking
  everything to read a single field.
- This crate is **stable**, not [0ver], post one-point-o.

[`packed_struct`]: https://lib.rs/crates/packed_struct
[`bondrewd`]: https://lib.rs/crates/bondrewd
[`arbitrary-int`]: https://lib.rs/crates/arbitrary-int
[`ux`]: https://lib.rs/crates/ux
[0ver]: https://0ver.org/
[`bitbybit`]: https://lib.rs/crates/bitbybit