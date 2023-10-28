// TODO(clean): remove the `cast_possible_truncation` ignore
#![allow(
    clippy::use_self,
    clippy::cast_possible_truncation,
    clippy::module_name_repetitions
)]
#![doc = include_str!("../README.md")]

pub use bimultimap::Bimultimap;
pub use bitmatrix::BitMatrix;
pub use bitset::Bitset;
#[cfg(feature = "enumset")]
pub use enum_bitmatrix::EnumBitMatrix;
#[cfg(feature = "enumset")]
pub use enum_multimap::EnumMultimap;
pub use index::Index;
pub use index_multimap::IndexMultimap;
pub use jagged_array::JaggedArray;
pub use jagged_bitset::JaggedBitset;
pub use jagged_vec::JaggedVec;
pub use packed_int_array::PackedIntArray;
pub use sorted_iter::assume::{AssumeSortedByItemExt, AssumeSortedByKeyExt};
pub use sorted_iter::{
    sorted_iterator::SortedByItem, sorted_pair_iterator::SortedByKey, SortedIterator,
    SortedPairIterator,
};

pub mod bimultimap;
pub mod bitmatrix;
pub mod bitset;
#[cfg(feature = "enumset")]
pub mod enum_bitmatrix;
#[cfg(feature = "enumset")]
pub mod enum_multimap;
// pub mod index_map;
pub mod index_multimap;
pub mod jagged_array;
pub mod jagged_bitset;
pub mod jagged_vec;
pub mod packed_int_array;
pub mod sorted;

/// Integer division rounded up.
const fn div_ceil(lhf: usize, rhs: usize) -> usize {
    (lhf + rhs - 1) / rhs
}
const fn safe_n_mask(n: u32) -> u32 {
    // https://stackoverflow.com/questions/52573447/creating-a-mask-with-n-least-significant-bits-set
    match n {
        n if n >= u32::BITS => u32::MAX,
        n => (1 << n) - 1,
    }
}
trait MostSignificantBit {
    fn most_significant_bit(&self) -> u32;
}
impl MostSignificantBit for u32 {
    fn most_significant_bit(&self) -> u32 {
        u32::BITS - self.leading_zeros()
    }
}
impl MostSignificantBit for usize {
    fn most_significant_bit(&self) -> u32 {
        usize::BITS - self.leading_zeros()
    }
}

/// Get an `usize` from `Self`.
#[rustfmt::skip]
#[allow(clippy::inline_always, clippy::unnecessary_cast)] // I mean, have you _seen_ what is being inlined?
mod index {
    /// A type that can be cast into an index.
    ///
    /// Note that `Index` types are assumed to **NOT** have a significant drop.
    pub trait Index {
        /// Get the index value of this type.
        fn get(&self) -> usize;
        /// Get the type from the index value.
        fn new(v: usize) -> Self;
    }
    impl Index for usize { #[inline(always)] fn get(&self) -> usize { *self as usize }#[inline(always)] fn new(v: usize) -> Self { v as Self } }
    impl Index for u8    { #[inline(always)] fn get(&self) -> usize { *self as usize }#[inline(always)] fn new(v: usize) -> Self { v as Self } }
    impl Index for u16   { #[inline(always)] fn get(&self) -> usize { *self as usize }#[inline(always)] fn new(v: usize) -> Self { v as Self } }
    impl Index for u32   { #[inline(always)] fn get(&self) -> usize { *self as usize }#[inline(always)] fn new(v: usize) -> Self { v as Self } }
    impl Index for u64   { #[inline(always)] fn get(&self) -> usize { *self as usize }#[inline(always)] fn new(v: usize) -> Self { v as Self } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msb() {
        assert_eq!(101_u32.most_significant_bit(), 7);
        assert_eq!(10_u32.most_significant_bit(), 4);
        assert_eq!(0b0000_u32.most_significant_bit(), 0);
        assert_eq!(0b0001_u32.most_significant_bit(), 1);
        assert_eq!(0b0010_u32.most_significant_bit(), 2);
        assert_eq!(0b0011_u32.most_significant_bit(), 2);
        assert_eq!(0b0100_u32.most_significant_bit(), 3);
        assert_eq!(0b0101_u32.most_significant_bit(), 3);
        assert_eq!(0b0110_u32.most_significant_bit(), 3);
        assert_eq!(0b0111_u32.most_significant_bit(), 3);
        assert_eq!(0b1000_u32.most_significant_bit(), 4);
        assert_eq!(0b1001_u32.most_significant_bit(), 4);
        assert_eq!(0b1010_u32.most_significant_bit(), 4);
        assert_eq!(0b1011_u32.most_significant_bit(), 4);
        assert_eq!(0b1100_u32.most_significant_bit(), 4);
        assert_eq!(0b1101_u32.most_significant_bit(), 4);
        assert_eq!(0b1110_u32.most_significant_bit(), 4);
        assert_eq!(0b1111_u32.most_significant_bit(), 4);
        assert_eq!(0b0100_0000_0000u32.most_significant_bit(), 11);
        assert_eq!(0b1000_0000_0000_0000u32.most_significant_bit(), 16);
        assert_eq!(0b0010_0000_0000_0000u32.most_significant_bit(), 14);
        assert_eq!(0xf000_0000u32.most_significant_bit(), 32);
        assert_eq!(0xffff_ffffu32.most_significant_bit(), 32);
        assert_eq!(0xffff_0000u32.most_significant_bit(), 32);
    }
}
