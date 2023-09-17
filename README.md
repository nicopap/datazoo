# Enter the Data Zoo

A collection of data structures interesting enough to put in free range pens.

## Features

This is a collection of [multimaps], [jagged arrays], [bit sets],
and combination thereof.

Highlights:

- A storage-agnostic `Bitset`.
- A storage-agnostic `JaggedArray`.
- A `PackedIntArray` similar to `bitpacking`, `bitvec` and other bitpacking
  rust crates.
- A bi-directional read-only multimap: `BimultiMap`
- [`enumset`]-keyed data structures.

See `rustdoc` documentation for details.

## Cargo Features

- `enumset`: enables the [`enumset`] dependency and the `EnumBitMatrix`
  `EnumMultimap` data structures
- `smallvec`: (off by default) Implement `bitset::ExtendBlocks` on `SmallVec`.

## Unique features

- `JaggedArray` is generic over storage types, which means you can take
  advantage of your own specialized `slice` impl such as `SmallVec`, or a fixed
  sze array stored on the stack (typically useful if you have a fixed height matrix).
- `Bitset` is _also_ generic over storage type, with the same implications.

## Limitations

- Data structures do not handle `Drop` types at all.
- Data structures are **untested with sizes `> u32::MAX`**
- Effort is made to panic in those situations though, but you never know
- Generally assumes `size_of(usize) >= size_of(u32)`
- Definitively not as tested and benchmarked as other similar crates.
- No `#[no_std]` but I don't see why this couldn't be added as a feature
- depends on `sorted-iter`, can't disable dependency.
- `Bitset` generally doesn't distinguish between "disabled within bound" and
  "out of bound".

## History

This was first built for the [`cuicui_richtext`] project, but became useful for
other things, so it got published separately.


## License

You may use `datazoo` under any of the following licenses:

- ZLib
- MIT
- Apache 2.0

[`enumset`]: https://lib.rs/crates/enumset
[multimaps]: https://en.wikipedia.org/wiki/Multimap
[jagged arrays]: https://en.wikipedia.org/wiki/Jagged_array
[bit sets]: https://en.wikipedia.org/wiki/Bit_array
[`cuicui_richtext`]: https://github.com/nicopap/cuicui