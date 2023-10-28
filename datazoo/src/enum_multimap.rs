//! A [multimap] optimized for [`EnumSetType`] keys.
//!
//! [multimap]: https://en.wikipedia.org/wiki/Multimap

use std::{fmt, marker::PhantomData, mem::size_of};

use enumset::{EnumSet, EnumSetType};

use crate::JaggedArray;

struct OwnAsRefSlice<const U: usize>(Box<[u32; U]>);
impl<const U: usize> AsRef<[u32]> for OwnAsRefSlice<U> {
    fn as_ref(&self) -> &[u32] {
        self.0.as_ref()
    }
}
/// A [multimap] stored in a [`JaggedArray`].
///
/// The key set need to be bound and exhaustively known at compile time,
/// ie: it must be an enum derived with `#[derive(EnumSetType)]`.
///
/// Use it as follow:
/// `EnumMultimap<MyEnumSet, ModifyIndex, { (MyEnumSet::BIT_WIDTH - 1) as usize }>`
///
/// [multimap]: https://en.wikipedia.org/wiki/Multimap
pub struct EnumMultimap<K: EnumSetType, V, const CLM: usize> {
    inner: JaggedArray<V, u32, OwnAsRefSlice<CLM>>,
    _key: PhantomData<K>,
}
impl<K: EnumSetType, V: fmt::Debug, const CLM: usize> fmt::Debug for EnumMultimap<K, V, CLM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("EnumMultimap").field(&self.inner).finish()
    }
}
impl<K: EnumSetType, V, const CLM: usize> EnumMultimap<K, V, CLM> {
    /// Iterate over all rows, as slice, keyed by elements present in `set`.
    pub fn all_rows(&self, set: EnumSet<K>) -> impl Iterator<Item = &[V]> + '_ {
        set.iter()
            .filter_map(|x| self.inner.get_row(x.enum_into_u32() as usize))
    }
    /// Get row slice for `key`.
    #[must_use]
    pub fn row(&self, key: K) -> &[V] {
        let index = key.enum_into_u32() as usize;
        // SAFETY: by construction, `K` has a value below CLM.
        unsafe { self.inner.get_row(index).unwrap_unchecked() }
    }
    /// Get `V` at exact `direct_index` ignoring row sizes,
    /// acts as if the whole array was a single row.
    ///
    /// `None` when `direct_index` is out of bound.
    #[must_use]
    pub fn get(&self, direct_index: usize) -> Option<&V> {
        self.inner.get(direct_index)
    }
}

/// Build a [`EnumMultimap`].
#[derive(Debug, Clone)]
pub struct Builder<K, V, const CLM: usize> {
    rows: Vec<Box<[V]>>,
    _key: PhantomData<K>,
}
impl<K: EnumSetType, V, const CLM: usize> Default for Builder<K, V, CLM> {
    fn default() -> Self {
        Self::new()
    }
}
impl<K: EnumSetType, V, const CLM: usize> Builder<K, V, CLM> {
    // Compile time error when `CLM` is not the correct value.
    // This works around a limitation of rust' type system,
    // where it is impossible to use associated constants in generic const position.
    const _COMPILE_TIME_ERROR: () = {
        assert!(K::BIT_WIDTH as usize == CLM + 1);
        assert!(size_of::<usize>() >= size_of::<u32>());
    };

    /// Create a new [`EnumMultimap`] builder.
    #[must_use]
    pub fn new() -> Self {
        Builder { rows: Vec::with_capacity(CLM), _key: PhantomData }
    }
    /// Insert provided `values` into `key` row.
    pub fn insert(&mut self, key: K, values: impl Iterator<Item = V>) {
        let row = key.enum_into_u32() as usize;
        self.rows.insert(row, values.collect());
    }
    /// Create the [`EnumMultimap`] from provided rows.
    #[must_use]
    pub fn build(self) -> EnumMultimap<K, V, CLM> {
        let mut end = 0;
        let mut ends = Box::new([0; CLM]);
        let mut data = Vec::new();
        for (i, values) in self.rows.into_iter().enumerate() {
            end += values.len() as u32;
            data.extend(values.into_vec());
            if i < CLM {
                ends[i] = end;
            }
        }
        // SAFETY:
        // - by construction, the ends are always increasing
        // - by construction, ends never grow beyond the total size of `data`.
        let inner = unsafe {
            JaggedArray::new(OwnAsRefSlice(ends), data.into_boxed_slice()).unwrap_unchecked()
        };
        EnumMultimap { inner, _key: PhantomData }
    }
}
