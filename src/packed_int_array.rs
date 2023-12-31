//! An [associative array] of small integers.
//!
//! [associative array]: https://en.wikipedia.org/wiki/Associative_array

use std::{fmt, marker::PhantomData};

use crate::{div_ceil, safe_n_mask, Bitset, Index, MostSignificantBit};

/// Parametrize [`PackedIntArray`] to implement equality in terms of `V` rather
/// than raw bit value.
///
/// The default `PackedIntArray` equality just compares the values as they are stored
/// in the indexmap.
///
/// However, if your `V`'s implementation of equality differs from basic integer
/// equality, you can use the `ValueEq` type as follow:
///
/// # Example
///
/// ```
/// use datazoo::{Index, PackedIntArray, packed_int_array::ValueEq};
///
/// #[derive(Debug, Clone, PartialEq)]
/// struct MyV(u32);
/// impl From<u32> for MyV { fn from(v: u32) -> Self {MyV(v)} }
/// impl Index for MyV { fn get(&self) -> usize {self.0 as usize} fn new(v: usize) -> Self { MyV(v as u32) } }
///
/// let mut map = PackedIntArray::<usize, MyV, ValueEq>::with_capacity(32, 32);
///
/// map.set(&1, &MyV(2));
/// map.set(&0, &MyV(5));
/// map.set(&31, &MyV(9));
///
/// let partial_map = map.clone();
///
/// map.set(&15, &MyV(1));
/// map.set(&14, &MyV(0));
///
/// let identical_map = map.clone();
///
/// assert_ne!(partial_map, map);
/// assert_eq!(identical_map, map);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueEq {}

/// An [associative array] of small integers.
///
/// A 1-to-(1|0) mapping of integers to integers, in packed storage.
/// It might help to think of this as a very compact `Vec<Option<V>>`.
///
/// See [`IndexMultimap`] for 1-to-N mapping.
///
/// # Design
///
/// A typical array of integers (consider `Vec<u32>`) takes 32 bits per entry,
/// a `PackedIntArray` only takes as much bit per entry as the largest entry.
///
/// ```text
/// // vec![12_u32, 2, 5, 8, 10, 9];
/// // size in memory: 24 stack bytes + 24 heap bytes
/// [cap; len; ptr] -> [ // binary
/// 0000_0000_0000_0000_0000_0000_0000_1100 // 12
/// 0000_0000_0000_0000_0000_0000_0000_0010 // 2
/// 0000_0000_0000_0000_0000_0000_0000_0101 // 5
/// 0000_0000_0000_0000_0000_0000_0000_1000 // 8
/// 0000_0000_0000_0000_0000_0000_0000_1010 // 10
/// 0000_0000_0000_0000_0000_0000_0000_1001 // 9
/// ]
/// // PackedIntArray<u32, u32> = [12, 2, 5, 8, 10, 9];
/// // size in memory: 24 stack bytes + 4 heap bytes
/// [value_width; len; ptr] -> [ // binary
/// 1100 // 12
/// 0010 // 2
/// 0101 // 5
/// 1000 // 8
/// 1010 // 10
/// 1001 // 9
/// 1111 // We use [u32] for storage, byte count is always
/// 1111 // multiple of 4, empty trailing slots are filled with 1s.
/// ]
/// ```
///
/// Now, take `Vec<Option<u32>>`. `[12, 2, 5, 8, 10, 9].map(Some)` takes twice
/// as much heap space. 48 bytes! `PackedIntArray` has special handling of
/// empty slots, so they take exactly as much space as filled slots.
///
/// We went from 48 bytes to 4 bytes. We basically can divide by **ten**
/// memory usage.
///
/// Note that while this may take less memory, it requires much more processing.
///
/// # `PartialEq` implementation
///
/// The default `PackedIntArray` equality just compares the values as they are stored
/// in the indexmap.
///
/// However, if your `V`'s implementation of equality differs from basic integer
/// equality, you can use the [`ValueEq`] type as described in the [`ValueEq`] docs.
///
/// # Example
///
/// ```
/// use datazoo::PackedIntArray;
///
/// let mut map = PackedIntArray::<usize, u32>::with_capacity(200, 100);
///
/// map.set(&10, &2);
/// map.set(&11, &5);
/// map.set(&19, &9);
/// map.set(&15, &1);
///
/// map.set(&31, &22);
/// map.set(&30, &20);
/// map.set(&32, &21);
///
/// assert_eq!(map.get(&10), Some(2));
/// assert_eq!(map.get(&32), Some(21));
///
/// // zero is distinct from no values
/// assert_eq!(map.get(&13), None);
/// map.set(&13, &0);
/// assert_eq!(map.get(&13), Some(0));
///
/// // values and indices set out of bound do nothing.
/// // `set` returns `None` if they are out of bound.
/// assert_eq!(map.set(&350, &3), None);
/// assert_eq!(map.get(&350), None);
///
/// assert_eq!(map.set(&31, &200), None);
/// assert_eq!(map.get(&31), Some(22));
///
/// // Note that not _all_ values over the provided max_key and max_value are
/// // forbidden. You can't rely on them being available, but neither can you
/// // rely on them being never set. (max_key = 199, max_value = 99)
/// assert_eq!(map.set(&200, &3), Some(()));
/// assert_eq!(map.get(&200), Some(3));
///
/// assert_eq!(map.set(&200, &111), Some(()));
/// assert_eq!(map.get(&200), Some(111));
///
/// // It is also possible to use `collect` to create an `IndexMap`
/// let other_map: PackedIntArray<usize, u32> = [
///     (10, 2),
///     (11, 5),
///     (19, 9),
///     (15, 1),
///     (31, 22),
///     (30, 20),
///     (32, 21),
///     (13, 0),
///     (200, 3),
///     (200, 111),
/// ].into_iter().collect();
///
/// assert_eq!(map, other_map);
/// ```
///
/// [`IndexMultimap`]: crate::IndexMultimap
/// [associative array]: https://en.wikipedia.org/wiki/Associative_array
#[derive(Clone)]
pub struct PackedIntArray<K: Index, V: From<u32>, Eq = ()> {
    /// A matrix of `max(K)` rows of `log₂(max(V) + 1)` bits, each row represents
    /// a single index.
    ///
    /// If all the bits of the row are set (`u32::MAX`), then it means
    /// the row is **empty**. (this allows `Value` values of 0)
    ///
    /// It might be useful to consider this as an array of integers of
    /// arbitrary bit witdth.
    indices: Bitset<Box<[u32]>>,
    value_width: usize,
    _tys: PhantomData<fn(K, V, Eq)>,
}
impl<K: Index, V: From<u32>, Eq> Default for PackedIntArray<K, V, Eq> {
    fn default() -> Self {
        PackedIntArray {
            indices: Bitset(Vec::new().into_boxed_slice()),
            value_width: 0,
            _tys: PhantomData,
        }
    }
}
impl<K: Index, V: From<u32>, Eq> PackedIntArray<K, V, Eq> {
    /// Initialize a [`PackedIntArray`] with static size.
    ///
    /// You can always insert:
    /// - Keys: `(0 ..= key_len-1)`
    /// - Values: `(0 ..= value_len-1)`
    ///
    /// In the case where either `key_len` or `value_len` is zero, you won't
    /// be able to insert anything. `set` will do nothing, `get` always returns `None`.
    ///
    /// When `value_len` equals `1`, this is equivalent to a [`Bitset`] with `key_len` bits.
    /// When `value_len` is between `u32::MAX / 2` and `u32::MAX`,
    /// this is equivalent to a `Vec<u32>`.
    ///
    /// Some larger values may be accepted.
    /// Specifically, the following will be accepted in the current implementation.
    /// **This is not stable, do not assume this will always be true**:
    ///
    /// - Values: any value smaller or equal to `2^vwidth - 1`.
    /// - Keys: `⌊LMO₃₂(max_key · vwidth) / vwidth⌋`
    ///
    /// Where:
    /// - `⌈x⌉ = ceil(x)`
    /// - `⌊x⌋ = floor(x)`
    /// - `LMOn(x) = n · ⌈x / n⌉` (lowest multiple of `n` over `x`)
    /// - `vwidth = ⌈log₂(max_value + 1)⌉` (width in bits of a value)
    #[must_use]
    pub fn with_capacity(key_len: usize, value_len: u32) -> Self {
        let vwidth = value_len.most_significant_bit() as usize;
        let bit_size = vwidth * key_len;
        let u32_size = div_ceil(bit_size, u32::BITS as usize);
        PackedIntArray {
            indices: Bitset(vec![u32::MAX; u32_size].into_boxed_slice()),
            value_width: vwidth,
            _tys: PhantomData,
        }
    }
    /// How many keys at most this contains.
    ///
    /// Unlike a `HashMap`, the capacity also represents the upper
    /// limit of key values (all keys are smaller than `capacity`).
    ///
    /// This might not be the `key_len` provided as argument to [`Self::with_capacity`],
    /// as the underlying array aligns the number of bits to the next multiple of 32.
    #[allow(clippy::unnecessary_lazy_evaluations)] // see comment
    #[must_use]
    pub fn capacity(&self) -> usize {
        let bit_len = self.indices.bit_len();
        // This prevents a division by zero when both bit_len and value_width are zero
        (bit_len != 0)
            .then(|| bit_len / self.value_width)
            .unwrap_or(0)
    }
    #[inline]
    fn row_offset(&self, index: usize) -> usize {
        index.get() * self.value_width
    }
    #[inline]
    fn value_mask(&self) -> Option<u32> {
        let shift = self.value_width as u32;
        (shift != 0).then(|| safe_n_mask(shift))
    }
    fn get_index(&self, index: usize) -> Option<V> {
        let offset = self.row_offset(index);
        let width = self.value_width as u32;
        let mask = self.value_mask()?;
        let value = mask & self.indices.n_at(width, offset)?;
        // != means the row is not empty
        (value != mask && index < self.capacity()).then(|| V::from(value))
    }
    /// Get the value associated with `index`, `None` if there isn't.
    #[inline]
    pub fn get(&self, index: &K) -> Option<V> {
        self.get_index(index.get())
    }
    /// Remove value associated with `key`. Afterward, calling `map.get(key)`
    /// will return `None`.
    pub fn remove(&mut self, key: &K) {
        let offset = self.row_offset(key.get());
        self.indices.extend(offset..offset + self.value_width);
    }
    /// Set value of `key` to `value`.
    ///
    /// Returns `None` if either `value` or `key` is out of bound.
    ///
    /// # Example
    ///
    /// ```
    /// # use datazoo::PackedIntArray;
    /// let mut map = PackedIntArray::<usize, u32>::with_capacity(200, 100);
    /// assert_eq!(map.get(&32), None);
    ///
    /// map.set(&32, &28);
    /// assert_eq!(map.get(&32), Some(28));
    ///
    /// map.set(&32, &0);
    /// map.set(&33, &12);
    /// assert_eq!(map.get(&32), Some(0));
    /// assert_eq!(map.get(&33), Some(12));
    /// ```
    #[inline]
    pub fn set(&mut self, key: &K, value: &V) -> Option<()>
    where
        V: Index,
    {
        let value = value.get() as u32;
        let key = key.get();
        let mask = self.value_mask()?;

        // either max value or larger than bitmask
        if key >= self.capacity() || value == mask || value & mask != value {
            return None;
        }
        let offset = self.row_offset(key);

        self.indices
            .disable_range(offset..offset + self.value_width);
        self.indices
            .extend(Bitset([value]).ones().map(|v| v + offset as u32));
        Some(())
    }
    /// Set value of `key` to `value`.
    ///
    /// Increase the size of the buffer if `value` is out of bound.
    /// If `key` is out of bound, does nothing and returns `None`.
    #[inline]
    pub fn set_expanding_values(&mut self, key: &K, value: &V) -> Option<()>
    where
        V: Index,
    {
        let value_u32 = value.get() as u32;
        let value_bits = value_u32.most_significant_bit();
        let width = self.value_width as u32;
        if value_bits > width || value_u32 == self.value_mask()? {
            let additional_bits = value_bits - width;
            let offset = |x: u32| x + x / width * additional_bits;
            let new_indices = self.indices.ones().map(offset);
            self.indices = new_indices.collect();
            self.value_width += additional_bits as usize;
        }
        self.set(key, value)
    }
    /// Iterate over all values.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (K, V)> + '_ {
        (0..self.capacity()).filter_map(|k| self.get_index(k).map(|v| (K::new(k), v)))
    }
    /// Iterate over all values (reversed).
    #[inline]
    pub fn rev_iter(&self) -> impl Iterator<Item = (K, V)> + '_ {
        (0..self.capacity())
            .rev()
            .filter_map(|k| self.get_index(k).map(|v| (K::new(k), v)))
    }
}
impl<K: Index, V: From<u32>> PartialEq for PackedIntArray<K, V> {
    fn eq(&self, other: &Self) -> bool {
        let min_len = self.indices.0.len().min(other.indices.0.len());
        let largest = if self.indices.0.len() == min_len { other } else { self };

        let common_identical = self.indices.0[..min_len] == other.indices.0[..min_len];
        let no_more = largest.indices.0[min_len..].iter().all(|v| *v == u32::MAX);
        common_identical && no_more
    }
}
impl<K: Index, V: From<u32> + PartialEq> PartialEq for PackedIntArray<K, V, ValueEq> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let max = self.capacity().max(other.capacity());
        (0..max).all(|k| self.get_index(k) == other.get_index(k))
    }
}

impl<K: Index, V: From<u32> + Index> FromIterator<(K, V)> for PackedIntArray<K, V> {
    /// Create a [`PackedIntArray`] where value at `k` will be `value` in `(key, value)`
    /// the last item where `key == k`.
    ///
    /// Note that all `K` and `V` will be dropped.
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut max_value = 0;
        let mut max_key = 0;

        let key_values = iter
            .into_iter()
            .map(|(k, v)| {
                max_key = max_key.max(k.get() + 1);
                max_value = max_value.max(v.get() + 1);
                (k, v)
            })
            .collect::<Box<[_]>>();

        let max_value = u32::try_from(max_value).unwrap();
        let mut map = PackedIntArray::with_capacity(max_key, max_value);

        for (key, value) in &*key_values {
            map.set(key, value);
        }
        map
    }
}
impl<K, V, Eq> fmt::Debug for PackedIntArray<K, V, Eq>
where
    K: Index + fmt::Debug,
    V: From<u32> + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_map();
        self.iter().for_each(|(k, v)| {
            list.entry(&k, &v);
        });
        list.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity() {
        let max_value = 127_u32;
        let max_key = 32 * 7;
        let map = PackedIntArray::<usize, u32>::with_capacity(max_key, max_value);
        assert_eq!(max_key, map.capacity());
    }
    #[test]
    fn compact_size() {
        let value_len = 127_u32;
        let key_len = 32 * 7;
        let mut map = PackedIntArray::<usize, u32>::with_capacity(key_len, value_len);

        let max_key = key_len - 1;
        let max_value = value_len - 1;
        assert_eq!(map.set(&max_key, &0), Some(()));
        assert_eq!(map.get(&max_key), Some(0));

        assert_eq!(map.set(&0, &max_value), Some(()));
        assert_eq!(map.get(&0), Some(max_value));
    }
    #[test]
    fn mini_size() {
        let mut map = PackedIntArray::<usize, u32>::with_capacity(0, 0);
        assert_eq!(map.indices.0.len(), 0);

        assert_eq!(map.set(&32, &0), None);
        assert_eq!(map.set(&0, &0), None);

        assert_eq!(map.get(&32), None);
        assert_eq!(map.get(&0), None);

        let mut map = PackedIntArray::<usize, u32>::with_capacity(0, u32::MAX);
        assert_eq!(map.indices.0.len(), 0);

        assert_eq!(map.set(&32, &0), None);
        assert_eq!(map.set(&0, &0), None);

        assert_eq!(map.get(&32), None);
        assert_eq!(map.get(&0), None);

        let mut map = PackedIntArray::<usize, u32>::with_capacity(u32::MAX as usize, 0);
        assert_eq!(map.indices.0.len(), 0);

        assert_eq!(map.set(&32, &0), None);
        assert_eq!(map.set(&0, &0), None);

        assert_eq!(map.get(&32), None);
        assert_eq!(map.get(&0), None);
    }
    #[test]
    fn size() {
        let len = 128;
        let mut map = PackedIntArray::<usize, u32>::with_capacity(len, u32::MAX);
        assert_eq!(map.indices.0.len(), len);

        assert_eq!(map.set(&32, &0xffff_ff00), Some(()));
        assert_eq!(map.get(&32), Some(0xffff_ff00));
        assert_eq!(map.set(&(len - 1), &0xffff_0000), Some(()));
        assert_eq!(map.get(&(len - 1)), Some(0xffff_0000));
    }
    #[test]
    fn expand_size() {
        let max_value = 127_u32;
        let max_key = 32 * 7;
        let mut map = PackedIntArray::<usize, u32>::with_capacity(max_key, max_value);
        assert_eq!(max_key, map.capacity());

        assert_eq!(map.set(&12, &100), Some(()));
        assert_eq!(map.set(&20, &101), Some(()));
        assert_eq!(map.set(&32, &102), Some(()));
        assert_eq!(map.set(&300, &103), None);
        assert_eq!(map.set(&32, &200), None);

        // Test single bit expension
        assert_eq!(map.set_expanding_values(&35, &200), Some(()));
        assert_eq!(map.set_expanding_values(&300, &200), None);
        assert_eq!(map.set(&13, &199), Some(()));

        assert_eq!(map.get(&12), Some(100));
        assert_eq!(map.get(&13), Some(199));
        assert_eq!(map.get(&20), Some(101));
        assert_eq!(map.get(&32), Some(102));
        assert_eq!(map.get(&35), Some(200));

        // multibit extension
        assert_eq!(map.set_expanding_values(&36, &1845), Some(()));

        assert_eq!(map.get(&12), Some(100));
        assert_eq!(map.get(&13), Some(199));
        assert_eq!(map.get(&20), Some(101));
        assert_eq!(map.get(&32), Some(102));
        assert_eq!(map.get(&35), Some(200));
        assert_eq!(map.get(&36), Some(1845));
    }
}
