# 0.7.0

- Update smallvec dep
- Use more precise 'thiserror' dep
- **NEW**: more `JaggedVec` API
  - `rows`: iterate rows
  - `empty` & `Default` impl: create an empty `JaggedVec`
  - `get_row`: non-panicking version of `row`
  - `extend_last_row`, `push`: Add elements to the last row
  - `clear`: Remove all rows from the `JaggedVec`
  - `pop_row`: Remove the last row, returning its content.
  - `Debug` impl: `JaggedVec` is now represented as a list of list
- **NEW, BREAKING**: `JaggedVec` (unlike `JaggedArray`) now can have a height of 0
- **NEW, BREAKING**: `JaggedVec::push_row` now returns `&mut Self` to make it easier
  to chain calls to build a `JaggedVec`.

# 0.6.0

Cleanup README and improve documentation.

Breaking renames:
- `raw_index_map` → `packed_int_array`, `RawIndexMap` → `PackedIntArray`
- `bitmultimap` → `bimultimap`, `BitMultimap` → `Bimultimap`

# 0.5.0

Add the `ExtendBlocks` trait and use it for `enable_bit_extending`. This allows
using the method on arbitrary types.

`ExtendBlocks` is implemented for:

- `Box<[u32]>`
- `Vec<u32>`
- `SmallVec<[u32; N]>` (behind feature flag)

Add `JaggedArrayRows: Clone`. `Copy` is not implemented, because `JaggedArrayRows`
is an iterator, and `Copy` iterators are confusing (it's also hard to implement).

Add `Bitset::ones()`, shortcut for `Bitset::ones_in_range(..)`.

# 0.4.0

Remove `From<usize>` bound on `K` in `RawIndexMap`. It was unneeded, because
`Index` trait already can perform this conversion.

# 0.3.0

Make `JaggedArray` generic over the storage kind, by adding the `VS` type
parameter. It makes the type definitively fairly difficult to gork, but it
enables things like subslicing.

# 0.2.0

Initial release.
