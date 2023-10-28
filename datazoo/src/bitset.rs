//! A slice of `u32` accessed on the bit level.

use std::{fmt, iter, ops::Range, ops::RangeBounds};

use sorted_iter::sorted_iterator::SortedByItem;

use crate::{div_ceil, safe_n_mask};

#[cfg(test)]
mod tests;

trait BlockT {
    const BITS64: usize;
}
impl BlockT for u32 {
    const BITS64: usize = u32::BITS as usize;
}

/// A slice of `u32` accessed on the bit level, see [wikipedia][bitset].
///
/// # Usage
///
/// `Bitset` is parametrized on the storage type, to let you chose whether
/// this needs to be a reference, a `Box`, a `Vec`, or even a 3rd party slice
/// type such as `SmallVec`.
///
/// Mutable methods are only available when the underlying storage allows
/// mutable access.
///
/// ```rust
/// use datazoo::Bitset;
/// let bunch_of_bits = [0xf0f0_00ff, 0xfff0_000f, 0xfff0_0f0f];
///
/// let as_array: Bitset<[u32; 3]> = Bitset(bunch_of_bits);
/// let mut as_vec: Bitset<Vec<u32>> = Bitset(bunch_of_bits.to_vec());
/// let as_slice: Bitset<&[u32]> = Bitset(&bunch_of_bits);
/// let as_box: Bitset<Box<[u32]>> = Bitset(Box::new(bunch_of_bits));
///
/// assert_eq!(
///     as_array.ones_in_range(5..91),
///     as_vec.ones_in_range(5..91),
/// );
/// assert_eq!(
///     as_vec.ones_in_range(5..91),
///     as_slice.ones_in_range(5..91),
/// );
/// assert_eq!(
///     as_slice.ones_in_range(5..91),
///     as_box.ones_in_range(5..91),
/// );
/// assert_eq!(
///     as_box.ones_in_range(5..91),
///     as_array.ones_in_range(5..91),
/// );
/// ```
///
/// To use mutable methods ([`Bitset::enable_bit`] is currently the only one),
/// the backing storage `B` must be mutable. Otherwise, you just can't use them.
///
/// ```compile_fail
/// # use datazoo::Bitset;
/// # let bunch_of_bits = [0xf0f0_00ff, 0xfff0_000f, 0xfff0_0f0f];
/// let as_slice: Bitset<&[u32]> = Bitset(&bunch_of_bits);
///
/// as_slice.enable_bit(11);
/// ```
///
/// `Vec<_>` and `&mut [_]` do support mutable access, so the following works:
///
/// ```
/// # use datazoo::Bitset;
/// # let mut bunch_of_bits = [0xf0f0_00ff, 0xfff0_000f, 0xfff0_0f0f];
/// let mut as_vec: Bitset<Vec<u32>> = Bitset(bunch_of_bits.to_vec());
/// let mut as_mut_slice: Bitset<&mut [u32]> = Bitset(&mut bunch_of_bits);
///
/// assert_eq!(
///     as_vec.ones_in_range(5..91),
///     as_mut_slice.ones_in_range(5..91),
/// );
/// as_vec.enable_bit(11);
///
/// assert_ne!(
///     as_vec.ones_in_range(5..91),
///     as_mut_slice.ones_in_range(5..91),
/// );
/// as_mut_slice.enable_bit(11);
///
/// assert_eq!(
///     as_vec.ones_in_range(5..91),
///     as_mut_slice.ones_in_range(5..91),
/// );
/// ```
///
/// [bitset]: https://en.wikipedia.org/wiki/Bit_array
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Bitset<B: AsRef<[u32]>>(pub B);

/// A dynamic size slice allowing mutable extension to its own size.
///
/// This is used by the [`Bitset::enable_bit_extending`] method.
///
/// This is implemented on `Vec`, and `SmallVec` with the `smallvec` feature
/// enabled.
///
/// This is also implemented for `Box<[u32]>`. Since `Bitset` doesn't
/// make the distinction between out of bound and disabled, it makes always more
/// sense to use `Box<[u32]>` over `Vec<u32>`.
pub trait ExtendBlocks: AsMut<[u32]> + AsRef<[u32]> {
    /// Add `extra_blocks` of zeroed `u32`s to this slice, so that the new length
    /// is `self.len() + extra_blocks`.
    fn extend_blocks(&mut self, extra_blocks: usize);
}

impl ExtendBlocks for Box<[u32]> {
    /// Extend this `Box<[u32]>` to `(old_len + extra_blocks).next_pow2()`.
    ///
    /// The extension algorithm mirrors that of the standard library `Vec`.
    fn extend_blocks(&mut self, extra_blocks: usize) {
        let old_len = self.len();
        let new_len = (old_len + extra_blocks).next_power_of_two().max(8);
        let mut self_vec = std::mem::take(self).into_vec();

        self_vec.extend(iter::repeat(0).take(new_len - old_len));
        *self = self_vec.into();
    }
}

impl ExtendBlocks for Vec<u32> {
    fn extend_blocks(&mut self, extra_blocks: usize) {
        self.extend(iter::repeat(0).take(extra_blocks));
    }
}
#[cfg(feature = "smallvec")]
impl<A: smallvec::Array<Item = u32>> ExtendBlocks for smallvec::SmallVec<A> {
    fn extend_blocks(&mut self, extra_blocks: usize) {
        self.extend(iter::repeat(0).take(extra_blocks));
    }
}

impl<B: ExtendBlocks> Bitset<B> {
    /// Enables bit at position `bit`, extending `B` if necessary.
    ///
    /// When [`Bitset::bit`] will be called next, it will always be `true`.
    ///
    /// # Example
    ///
    /// ```
    /// # use datazoo::Bitset;
    /// let mut as_vec = Bitset(vec![]);
    /// assert!(as_vec.enable_bit(64).is_none());
    /// assert_eq!(as_vec.0.len(), 0);
    ///
    /// as_vec.enable_bit_extending(73);
    ///
    /// assert!(as_vec.bit(73));
    /// assert!(as_vec.enable_bit(64).is_some());
    /// assert!(as_vec.bit(64));
    /// assert_eq!(as_vec.0.len(), 3);
    /// ```
    /// Note that you can use this with `Box<[u32]>`:
    /// ```
    /// # use datazoo::Bitset;
    /// let mut as_box = Bitset(Box::<[u32]>::default());
    /// as_box.enable_bit_extending(73);
    /// assert!(as_box.bit(73));
    /// assert!(as_box.enable_bit(64).is_some());
    /// assert!(as_box.bit(64));
    /// ```
    pub fn enable_bit_extending(&mut self, bit: usize) {
        let block = bit / u32::BITS64;
        let offset = bit % u32::BITS64;

        let blocks_len = self.0.as_ref().len();
        if block >= blocks_len {
            let extra_blocks = block - blocks_len + 1;
            self.0.extend_blocks(extra_blocks);
        }
        let blocks = self.0.as_mut();
        blocks[block] |= 1 << offset;
    }
}

impl<B: AsRef<[u32]> + AsMut<[u32]>> Bitset<B> {
    /// Enables bit at position `bit`.
    ///
    /// Returns `None` and does nothing if `bit` is out of range.
    ///
    /// When [`Bitset::bit`] will be called next, it will be `true`
    /// if this returned `Some`.
    ///
    /// # Example
    ///
    /// ```
    /// # use datazoo::Bitset;
    /// let mut bitset = Bitset([0, 0, 0]);
    /// assert_eq!(bitset.bit(12), false);
    /// assert_eq!(bitset.bit(54), false);
    ///
    /// bitset.enable_bit(12);
    /// assert_eq!(bitset.bit(12), true);
    ///
    /// bitset.enable_bit(54);
    /// assert_eq!(bitset.bit(54), true);
    /// ```
    #[inline]
    pub fn enable_bit(&mut self, bit: usize) -> Option<()> {
        let block = bit / u32::BITS64;
        let offset = bit % u32::BITS64;

        self.0.as_mut().get_mut(block).map(|block| {
            *block |= 1 << offset;
        })
    }
    /// Disables bit at position `bit`.
    ///
    /// Returns `None` and does nothing if `bit` is out of range.
    ///
    /// When [`Bitset::bit`] will be called next, it will always return `false`.
    ///
    /// # Example
    ///
    /// ```
    /// # use datazoo::Bitset;
    /// let mut bitset = Bitset([0, 0, 0]);
    /// assert_eq!(bitset.bit(73), false);
    ///
    /// bitset.enable_bit(73);
    /// assert_eq!(bitset.bit(73), true);
    ///
    /// bitset.disable_bit(73);
    /// assert_eq!(bitset.bit(73), false);
    /// ```
    #[inline]
    pub fn disable_bit(&mut self, bit: usize) -> Option<()> {
        let block = bit / u32::BITS64;
        let offset = bit % u32::BITS64;

        self.0.as_mut().get_mut(block).map(|block| {
            *block &= !(1 << offset);
        })
    }
    /// Disables all bits in given range.
    ///
    /// # Example
    ///
    /// ```
    /// # use datazoo::Bitset;
    /// # use std::ops::Not;
    /// let mut bitset = Bitset(vec![0xffff_ffff, 0xffff_ffff, 0xffff_ffff]);
    ///
    /// bitset.disable_range(0..16);
    /// bitset.disable_range(35..54);
    ///
    /// assert!(bitset.bit(0).not());
    /// assert!(bitset.bit(16));
    /// assert!(bitset.bit(35).not());
    /// assert!(bitset.bit(53).not());
    /// ```
    #[inline]
    pub fn disable_range(&mut self, range: Range<usize>) {
        range.for_each(|i| {
            self.disable_bit(i);
        });
    }
}
impl<B: AsRef<[u32]>> Bitset<B> {
    /// How many bits in this array?
    ///
    /// Note that this will always return a multiple of 32.
    ///
    /// # Example
    ///
    /// ```
    /// # use datazoo::Bitset;
    /// let bitset = Bitset(&[0x0000_0000, 0x0000_0000, 0x0000_0000]);
    /// assert_eq!(bitset.bit_len(), 32 * 3);
    ///
    /// assert_eq!(Bitset(vec![0x0000_1000]).bit_len(), 32);
    ///
    /// assert_eq!(Bitset([]).bit_len(), 0);
    /// ```
    #[inline]
    pub fn bit_len(&self) -> usize {
        self.0.as_ref().len() * u32::BITS64
    }
    /// True if bit at `at` is enabled, false if out of bound or disabled.
    #[inline]
    pub fn bit(&self, at: usize) -> bool {
        let block = at / u32::BITS64;
        let offset = (at % u32::BITS64) as u32;
        let offset = 1 << offset;
        let Some(block) = self.0.as_ref().get(block) else {
            return false;
        };

        block & offset == offset
    }
    /// Returns the 32 bits in the bitset starting at `at`.
    ///
    /// # Errors
    /// Returns an `Err` with a truncated value if `at + 32` is larger than the bitset.
    ///
    /// # Example
    /// ```
    /// # use datazoo::Bitset;
    /// let bitset = Bitset(&[0xf0f0_00ff, 0xfff0_000f, 0xfff0_0f0f]);
    ///
    /// assert_eq!(bitset.u32_at(0),  Ok(0xf0f0_00ff));
    /// assert_eq!(bitset.u32_at(4),  Ok(0xff0f_000f));
    /// assert_eq!(bitset.u32_at(16), Ok(0x000f_f0f0));
    /// assert_eq!(bitset.u32_at(64), Ok(0xfff0_0f0f));
    ///
    /// assert_eq!(bitset.u32_at(96), Err(0));
    /// assert_eq!(bitset.u32_at(80), Err(0xfff0));
    /// ```
    #[inline]
    #[allow(clippy::similar_names)] // foo_1 is distinct from bar_0 fairly clearly
    pub fn u32_at(&self, at: usize) -> Result<u32, u32> {
        let block = at / u32::BITS64;
        let offset = (at % u32::BITS64) as u32;

        if offset == 0 {
            self.0.as_ref().get(block).copied().ok_or(0)
        } else {
            let inset = u32::BITS - offset;
            let msb_0 = self.0.as_ref().get(block).map_or(0, |&t| t) >> offset;
            let lsb_1 = self.0.as_ref().get(block + 1).map_or(0, |&t| t) << inset;

            let mask = safe_n_mask(inset);

            let spills_out = at + 32 > self.bit_len();
            let ctor = if spills_out { Err } else { Ok };
            ctor((msb_0 & mask) | (lsb_1 & !mask))
        }
    }
    /// Like [`Self::u32_at`], but limited to `n` bits. `n <= 32`.
    ///
    /// Returns `None` if `at + n` is larger than the bitset.
    #[inline]
    #[allow(clippy::similar_names)] // foo_1 is distinct from bar_0 fairly clearly
    pub fn n_at(&self, n: u32, at: usize) -> Option<u32> {
        // TODO(perf): use slice::align_to::<u64>
        let block = at / u32::BITS64;
        let offset = (at % u32::BITS64) as u32;

        let n_mask = safe_n_mask(n);

        if at + n as usize > self.bit_len() {
            None
        } else if offset + n <= 32 {
            let value = *self.0.as_ref().get(block)?;
            Some((value >> offset) & n_mask)
        } else {
            let inset = u32::BITS - offset;
            let msb_0 = self.0.as_ref().get(block)? >> offset;
            let lsb_1 = self.0.as_ref().get(block + 1)?.wrapping_shl(inset);

            let mask = safe_n_mask(inset);

            let value = (msb_0 & mask) | (lsb_1 & !mask);
            Some(value & n_mask)
        }
    }
    /// Same as [`self.ones_in_range(..)`].
    ///
    /// # Example
    /// ```
    /// # use datazoo::Bitset;
    /// let bitset = Bitset(&[0xf0f0_00ff, 0xfff0_000f, 0xfff0_0f0f]);
    ///
    /// assert_eq!(bitset.ones(), bitset.ones_in_range(..));
    /// ```
    ///
    /// [`self.ones_in_range(..)`]: Bitset::ones_in_range
    #[inline]
    pub fn ones(&self) -> Ones {
        let blocks = self.0.as_ref();
        let (bitset, remaining_blocks) = blocks.split_first().map_or((0, blocks), |(b, r)| (*b, r));
        Ones { block_idx: 0, crop: 0, bitset, remaining_blocks }
    }
    /// Get an iterator over the index of enabled bits within provided `range`.
    #[inline]
    pub fn ones_in_range(&self, range: impl RangeBounds<usize>) -> Ones {
        let start = match range.start_bound() {
            std::ops::Bound::Included(start) => *start,
            std::ops::Bound::Excluded(start) => *start + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(end) => *end + 1,
            std::ops::Bound::Excluded(end) => *end,
            std::ops::Bound::Unbounded => self.bit_len(),
        };

        // the offset to "crop" the bits at the edges of the [u32]
        let crop = Range {
            start: (start % u32::BITS64) as u32,
            end: (end % u32::BITS64) as u32,
        };
        // The indices of Blocks of [u32] (ie: NOT bits) affected by range
        let range = Range {
            start: start / u32::BITS64,
            end: div_ceil(end, u32::BITS64),
        };
        let all_blocks = &self.0.as_ref()[range.clone()];

        let (mut bitset, remaining_blocks) = all_blocks
            .split_first()
            .map_or((0, all_blocks), |(b, r)| (*b, r));

        bitset &= ((1 << crop.start) - 1) ^ u32::MAX;
        if remaining_blocks.is_empty() && crop.end != 0 {
            bitset &= (1 << crop.end) - 1;
        }
        Ones {
            block_idx: range.start as u32,
            crop: crop.end,

            bitset,
            remaining_blocks,
        }
    }
}
impl<B: AsRef<[u32]>> fmt::Debug for Bitset<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, block) in self.0.as_ref().iter().enumerate() {
            if i != 0 {
                write!(f, "_")?;
            }
            write!(f, "{block:08x}")?;
        }
        write!(f, "]")?;
        Ok(())
    }
}
impl<'a, B: AsRef<[u32]>> IntoIterator for &'a Bitset<B> {
    type Item = u32;
    type IntoIter = Ones<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.ones_in_range(0..self.bit_len())
    }
}
impl Extend<u32> for Bitset<Vec<u32>> {
    #[inline]
    fn extend<T: IntoIterator<Item = u32>>(&mut self, iter: T) {
        iter.into_iter()
            .for_each(|bit| self.enable_bit_extending(bit as usize));
    }
}
impl Extend<usize> for Bitset<Vec<u32>> {
    #[inline]
    fn extend<T: IntoIterator<Item = usize>>(&mut self, iter: T) {
        iter.into_iter()
            .for_each(|bit| self.enable_bit_extending(bit));
    }
}
impl Extend<u32> for Bitset<Box<[u32]>> {
    /// Add the iterator items to the `Bitset`, will **not** increase the
    /// bitset size.
    #[inline]
    fn extend<T: IntoIterator<Item = u32>>(&mut self, iter: T) {
        iter.into_iter().for_each(|bit| {
            self.enable_bit(bit as usize);
        });
    }
}
impl Extend<usize> for Bitset<Box<[u32]>> {
    /// Add the iterator items to the `Bitset`, will **not** increase the
    /// bitset size.
    #[inline]
    fn extend<T: IntoIterator<Item = usize>>(&mut self, iter: T) {
        iter.into_iter().for_each(|bit| {
            self.enable_bit(bit);
        });
    }
}
impl FromIterator<u32> for Bitset<Box<[u32]>> {
    fn from_iter<T: IntoIterator<Item = u32>>(iter: T) -> Self {
        let acc: Bitset<Vec<_>> = iter.into_iter().collect();
        Bitset(acc.0.into_boxed_slice())
    }
}
impl FromIterator<u32> for Bitset<Vec<u32>> {
    fn from_iter<T: IntoIterator<Item = u32>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let mut acc = Bitset(Vec::new());
        acc.extend(iter);
        acc
    }
}
impl FromIterator<usize> for Bitset<Box<[u32]>> {
    fn from_iter<T: IntoIterator<Item = usize>>(iter: T) -> Self {
        let acc: Bitset<Vec<_>> = iter.into_iter().collect();
        Bitset(acc.0.into_boxed_slice())
    }
}
impl FromIterator<usize> for Bitset<Vec<u32>> {
    fn from_iter<T: IntoIterator<Item = usize>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let mut acc = Bitset(Vec::new());
        acc.extend(iter);
        acc
    }
}

// TODO(perf): consider swapping block_idx, crop: u16
// or even a compact u26|u6 because `crop` can at most be `32`
/// Iterator over the enables bits of the subset of a [`Bitset`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ones<'a> {
    /// Index in u32 of `bitset`.
    block_idx: u32,
    /// How many bits to keep in the last block.
    crop: u32,

    bitset: u32,
    remaining_blocks: &'a [u32],
}
impl Iterator for Ones<'_> {
    type Item = u32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.bitset == 0 {
            let Some((&bitset, remaining_blocks)) = self.remaining_blocks.split_first() else {
                return None;
            };
            self.bitset = bitset;
            self.remaining_blocks = remaining_blocks;

            if self.remaining_blocks.is_empty() && self.crop != 0 {
                self.bitset &= (1 << self.crop) - 1;
            }
            self.block_idx += 1;
        }
        let t = self.bitset & 0_u32.wrapping_sub(self.bitset);
        let r = self.bitset.trailing_zeros();
        self.bitset ^= t;
        Some(self.block_idx * u32::BITS + r)
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let bitset_ones = self.bitset.count_ones();

        let Some((last, slice)) = self.remaining_blocks.split_last() else {
            return (bitset_ones as usize, Some(bitset_ones as usize));
        };
        let ones: u32 = slice.iter().map(|b| b.count_ones()).sum();
        let trailing_bits = last & !((1 << self.crop) - 1);
        let trailing_bits = trailing_bits.count_ones();

        let exact_size = (bitset_ones + ones + trailing_bits) as usize;
        (exact_size, Some(exact_size))
    }
}
impl ExactSizeIterator for Ones<'_> {}

impl SortedByItem for Ones<'_> {}

impl Ones<'_> {
    // TODO(BUG): not true when `Ones` is partially consumed, or starts not at a u32 block
    /// True if all items in the `Ones` is enabled (ie: iteration is a list of successors)
    ///
    /// # Bug
    /// This doesn't work if the start of range is not a multiple of `32`.
    ///
    /// # Example
    /// ```
    /// # use datazoo::Bitset;
    /// let bitset = Bitset(&[0xf0f0_00ff, 0xffff_ffff, 0xfff0_0f0f]);
    ///
    /// assert!(bitset.ones_in_range(32..64).all_one());
    /// assert!(bitset.ones_in_range(0..8).all_one());
    /// ```
    #[must_use]
    pub fn all_one(self) -> bool {
        let Some((last, slice)) = self.remaining_blocks.split_last() else {
            let mask = (1 << self.crop) - 1;
            return (self.bitset & mask) == mask;
        };

        let bitset_ones = self.bitset.count_ones() == self.bitset.trailing_ones();
        let prefix_ones = slice.iter().fold(true, |acc, &b| acc & (b == u32::MAX));
        let mask = (1 << self.crop) - 1;
        let tail_ones = (last & mask) == mask;
        bitset_ones && prefix_ones && tail_ones
    }
}
