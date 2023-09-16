//! A [multimap] optimized for [`EnumSetType`] keys.
//!
//! [multimap]: https://en.wikipedia.org/wiki/Multimap

use std::{fmt, marker::PhantomData, mem::size_of};

use enumset::{EnumSet, EnumSetType};

use crate::{jagged_array, JaggedArray};

struct OwnAsRefSlice<const U: usize>(Box<[u32; U]>);
impl<const U: usize> AsRef<[u32]> for OwnAsRefSlice<U> {
    fn as_ref(&self) -> &[u32] {
        self.0.as_ref()
    }
}
/// A [multimap] stored in a [`JaggedConstRowArray`].
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
    pub fn all_rows(&self, set: EnumSet<K>) -> impl Iterator<Item = &[V]> + '_ {
        set.iter()
            .filter_map(|x| self.inner.get_row(x.enum_into_u32() as usize))
    }
    #[must_use]
    pub fn row(&self, key: K) -> &[V] {
        let index = key.enum_into_u32() as usize;
        self.inner.row(index)
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

#[derive(Debug, Clone)]
pub struct Builder<K, V, const CLM: usize> {
    pub rows: Vec<Box<[V]>>,
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

    #[must_use]
    pub fn new() -> Self {
        Builder { rows: Vec::with_capacity(CLM), _key: PhantomData }
    }
    pub fn insert(&mut self, key: K, values: impl Iterator<Item = V>) {
        let row = key.enum_into_u32() as usize;
        self.rows.insert(row, values.collect());
    }
    pub fn build(self) -> Result<EnumMultimap<K, V, CLM>, jagged_array::Error> {
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
        let inner = JaggedArray::new(OwnAsRefSlice(ends), data.into_boxed_slice())?;
        Ok(EnumMultimap { inner, _key: PhantomData })
    }
}
