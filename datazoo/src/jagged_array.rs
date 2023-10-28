//! A variable length matrix optimized for read-only rows.

use std::ops::Bound::{Excluded, Included, Unbounded};
use std::{fmt, marker::PhantomData, ops::RangeBounds};

use thiserror::Error;

use crate::Index;

/// [`JaggedArray::new`] construction error.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error {
    /// An `end` in `ends` was lower than a previous one.
    #[error(
        "Cannot build JaggedArray: `ends` represents the end of each row in `data`, \
        it should be monotonically increasing. \
        Found `end` at position {i} lower than `end` at position {}", .i - 1
    )]
    BadEnd { i: usize },
    /// An `end` in `ends` was too large.
    #[error(
        "Cannot build JaggedArray: `ends` represents the end of each row in `data`, \
        Yet, `end` at position {i} ({end}) is larger than the length of data ({len})"
    )]
    TooLongEnd { i: usize, len: usize, end: usize },
}

/// A matrix of variable length row.
///
/// # Limitation
///
/// - A `JaggedArray` has at least one row, even if it is an empty row.
/// - This is a read-only data structure, Once a `JaggedArray` is built,
///   it's impossible to mutate it.
///
/// # Design
///
/// Instead of storing a `Vec<Vec<V>>`, `JaggedArray<V>` stores (1) an array of
/// indices of slice ends (2) a single `Vec<V>`.
///
/// The API abstracts this design and pretends fairly successfully that we have
/// an array of arrays underneath.
///
/// # Genericity
///
/// `JaggedArray` is generic over the index type. By default, it is `Box<[u32]>`,
/// but you can swap it to anything you like depending on your use case.
///
/// For example, you can store a fixed-height array for the same stack space
/// as the default `Box<[u32]>` as follow:
/// ```
/// use datazoo::JaggedArray;
///
/// let my_strs = vec!["one", "five", "ten", "eleven!", "fifth", "potato", "42", "twenth"];
/// // This has 9 rows, and all but the last row have a maximum size of 2¹⁶
/// let compact_array = JaggedArray::<&str, u16, [u16; 8]>::new([0; 8], my_strs.into());
/// ```
#[derive(PartialEq, Eq, Clone)]
pub struct JaggedArray<V, I: Index = u32, E: AsRef<[I]> = Box<[I]>, VS: AsRef<[V]> = Box<[V]>> {
    ends: E,
    data: VS,
    _i: PhantomData<fn([I], [V])>,
}

impl<V, I: Index, E: AsRef<[I]>, VS: AsRef<[V]>> JaggedArray<V, I, E, VS> {
    /// How many cells are contained in this `JaggedArray`.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.as_ref().len()
    }
    /// Is this array empty (no cells, it has at least one empty row).
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.as_ref().is_empty()
    }
    /// How many rows this `JaggedArray` has.
    #[inline]
    #[must_use]
    pub fn height(&self) -> usize {
        self.ends.as_ref().len() + 1
    }
    /// Create a [`JaggedArray`] of ` + 1` rows, values of `ends` are the
    /// end indicies (exclusive) of each row in `data`.
    ///
    /// Consider using [`jagged_array::Builder`] instead of `new` for a less
    /// error-prone version, in case `E = Box<[I]>`.
    ///
    /// Note that the `0` index and the last index should be elided.
    /// The last row will be the values between the last `end` in `ends` and
    /// the total size of the `data` array.
    ///
    /// # Errors
    /// - An `ends[i] > data.len()`
    /// - An `ends[i+1] < ends[i]`
    ///
    /// # Example
    /// ```rust
    /// use datazoo::JaggedArray;
    ///
    /// let ends = [0_u32, 0, 3, 4, 7, 9, 10, 10];
    /// let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 32];
    /// let jagged = JaggedArray::new(ends, data.into()).unwrap();
    /// let iliffe = jagged.into_vecs();
    ///
    /// assert_eq!(iliffe.len(), ends.len() + 1);
    /// assert_eq!(
    ///     iliffe,
    ///     vec![
    ///         vec![],
    ///         vec![],
    ///         vec![0, 1, 2],
    ///         vec![3],
    ///         vec![4, 5, 6],
    ///         vec![7, 8],
    ///         vec![9],
    ///         vec![],
    ///         vec![11, 32],
    ///     ],
    /// );
    /// ```
    ///
    /// [`jagged_array::Builder`]: Builder
    pub fn new(ends: E, data: VS) -> Result<Self, Error> {
        let mut previous_end = I::new(0);
        let last_end = data.as_ref().len();
        for (i, end) in ends.as_ref().iter().enumerate() {
            if end.get() > last_end {
                return Err(Error::TooLongEnd { i, len: last_end, end: end.get() });
            }
            if end.get() < previous_end.get() {
                return Err(Error::BadEnd { i });
            }
            previous_end = I::new(end.get());
        }
        Ok(Self { ends, data, _i: PhantomData })
    }
    /// Get `V` at exact `direct_index` ignoring row sizes,
    /// acts as if the whole array was a single row.
    ///
    /// `None` when `direct_index` is out of bound.
    ///
    /// # Example
    ///
    /// ```rust
    /// use datazoo::JaggedArray;
    ///
    /// let ends = &[0_u32, 0, 3, 4, 7, 9, 10, 10];
    /// let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9].into_boxed_slice();
    /// let jagged = JaggedArray::new(ends, data).unwrap();
    ///
    /// assert_eq!(jagged.get(4), Some(&4));
    /// ```
    #[inline]
    #[must_use]
    pub fn get(&self, direct_index: usize) -> Option<&V> {
        self.data.as_ref().get(direct_index)
    }

    /// Get slice to row at given `index`.
    ///
    /// # Panics
    /// See [`JaggedArray::get_row`] for an example and a non-panicking version.
    #[must_use]
    pub fn row(&self, index: usize) -> &[V] {
        self.get_row(index).unwrap()
    }
    /// Get row slice at given `index`.
    ///
    /// Returns `None` if `index` is out of bound (`index >= self.height()`).
    ///
    /// # Example
    /// ```rust
    /// let array = datazoo::jagged_array::Builder::<i64>::new()
    ///     .add_row([1, 2, 3]).add_row([4, 5, 6]).add_row([]).add_row([7, 8, 9])
    ///     .build();
    ///
    /// assert_eq!(array.get_row(1), Some(&[4, 5, 6][..]));
    /// assert_eq!(array.get_row(4), None);
    /// ```
    #[must_use]
    pub fn get_row(&self, index: usize) -> Option<&[V]> {
        self.get_rows(index..=index)
    }
    /// Same as [`JaggedArray::row`], but for a range of rows instead of individual rows.
    ///
    /// See more details at [`JaggedArray::get_rows`].
    ///
    /// # Panics
    /// If the range is out of bounds.
    #[must_use]
    pub fn rows(&self, range: impl RangeBounds<usize>) -> &[V] {
        self.get_rows(range).unwrap()
    }
    /// Same as [`JaggedArray::get_row`], but for a range of rows instead of individual rows.
    ///
    /// Returns `None` if the range is out of bound.
    ///
    /// # Example
    /// ```rust
    /// let array = datazoo::jagged_array::Builder::<i64>::new()
    ///     .add_row([1, 2, 3]).add_row([4, 5, 6]).add_row([]).add_row([7, 8, 9])
    ///     .build();
    ///
    /// assert_eq!(array.get_rows(..), Some(&[1, 2, 3, 4, 5, 6, 7, 8, 9][..]));
    /// assert_eq!(array.get_rows(2..), Some(&[7, 8, 9][..]));
    /// assert_eq!(array.get_rows(2..3), Some(&[][..]));
    /// ```
    #[inline]
    #[must_use]
    pub fn get_rows(&self, range: impl RangeBounds<usize>) -> Option<&[V]> {
        let ends = self.ends.as_ref();
        let get_end = |i| match i {
            n if n == ends.len() => Some(self.len()),
            n if n >= ends.len() => None,
            n => ends.get(n).map(I::get),
        };
        let start = match range.start_bound() {
            Included(0) | Unbounded => 0,
            Included(&start) => get_end(start - 1)?,
            Excluded(&start) => get_end(start)?,
        };
        let end = match range.end_bound() {
            Excluded(0) => 0,
            Excluded(&end) => get_end(end - 1)?,
            Included(&end) => get_end(end)?,
            Unbounded => self.len(),
        };
        if start > end {
            return None;
        }
        self.data.as_ref().get(start..end)
    }
    /// Iterate over every individual row slices of this `JaggedArray`.
    pub const fn rows_iter(&self) -> JaggedArrayRows<V, I, E, VS> {
        JaggedArrayRows { array: self, row: 0 }
    }
}

impl<V, I: Index, E: AsRef<[I]>> JaggedArray<V, I, E> {
    /// Turn this compact jagged array into a sparse representation.
    ///
    /// The returned `Vec<Vec<V>>` is an [Iliffe vector]. Iterating over it will
    /// be much slower than iterating over `JaggedArray`, but extending individual
    /// rows is much less costly.
    ///
    /// [Iliffe vector]: https://en.wikipedia.org/wiki/Iliffe_vector
    #[must_use]
    pub fn into_vecs(self) -> Vec<Vec<V>> {
        let Self { ends, data, .. } = self;
        let ends = ends.as_ref();
        let mut data = data.into_vec();

        let mut iliffe = Vec::with_capacity(ends.len());
        let mut last_end = 0;

        // TODO(perf): this is slow as heck because each drain needs to move
        // forward the end of the `data` vec, if we reverse ends here, we can
        // skip the nonsense.
        for end in ends {
            let size = end.get() - last_end;
            iliffe.push(data.drain(..size).collect());
            last_end = end.get();
        }
        iliffe.push(data);
        iliffe
    }
}
impl<V: fmt::Debug, I: Index, E: AsRef<[I]>> fmt::Debug for JaggedArray<V, I, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut full_array = f.debug_list();
        for row in self.rows_iter() {
            full_array.entry(&row);
        }
        full_array.finish()
    }
}

//
// `JaggedArrayRows`
//

/// Iterator over rows of a [`JaggedArray`].
pub struct JaggedArrayRows<
    'j,
    V,
    I: Index = u32,
    E: AsRef<[I]> = Box<[I]>,
    VS: AsRef<[V]> = Box<[V]>,
> {
    array: &'j JaggedArray<V, I, E, VS>,
    row: usize,
}

impl<'j, V, I: Index, E: AsRef<[I]>, VS: AsRef<[V]>> Clone for JaggedArrayRows<'j, V, I, E, VS> {
    fn clone(&self) -> Self {
        Self { array: self.array, row: self.row }
    }
}

impl<'j, V, I: Index, E: AsRef<[I]>> Iterator for JaggedArrayRows<'j, V, I, E> {
    type Item = &'j [V];

    fn next(&mut self) -> Option<Self::Item> {
        self.row += 1;
        self.array.get_row(self.row - 1)
    }
}

//
// `Builder`
//

/// Constructor for a [`JaggedArray`].
///
/// Note that only `JaggedArray`s with a `Box<[I]>` ends buffer (`E` type
/// parameter) can be constructed from a `Builder`.
///
/// To build a `JaggedArray` with arbitrary ends buffer, use [`JaggedArray::new`].
pub struct Builder<V, I = u32> {
    last_end: Option<I>,
    ends: Vec<I>,
    data: Vec<V>,
}
impl<V, I: Index> Default for Builder<V, I> {
    fn default() -> Self {
        Builder { last_end: None, ends: Vec::new(), data: Vec::new() }
    }
}
impl<V, I: Index> Builder<V, I> {
    /// Create a new [`JaggedArray`] builder.
    ///
    /// Use [`Self::add_row`] to add rows, and [`Self::build`] to get the `JaggedArray`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Create a new [`JaggedArray`] builder with pre-allocated buffers.
    ///
    /// Use [`Self::add_row`] to add rows, and [`Self::build`] to get the `JaggedArray`.
    #[must_use]
    pub fn new_with_capacity(row_count: usize, data_len: usize) -> Self {
        Builder {
            last_end: None,
            ends: Vec::with_capacity(row_count),
            data: Vec::with_capacity(data_len),
        }
    }
    /// Add a single element to the current row.
    ///
    /// Use [`Self::add_row`] to "commit" elements to a row, for example with
    /// `builder.add_row(std::iter::empty)`.
    pub fn add_elem(&mut self, elem: V) -> &mut Self {
        self.data.push(elem);
        self
    }
    /// Add all elements in `row` to the current row and mark it as a distinct
    /// row in the resulting [`JaggedArray`].
    pub fn add_row(&mut self, row: impl IntoIterator<Item = V>) -> &mut Self {
        self.data.extend(row);
        if let Some(last_end) = self.last_end.replace(I::new(self.data.len())) {
            self.ends.push(last_end);
        }
        self
    }
    /// Complete this [`JaggedArray`], consuming this `Builder`.
    #[must_use]
    pub fn build(&mut self) -> JaggedArray<V, I> {
        let ends = std::mem::take(&mut self.ends);
        let data = std::mem::take(&mut self.data);
        JaggedArray {
            ends: ends.into(),
            data: data.into(),
            _i: PhantomData,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_row() {
        let array = Builder::<i64>::new()
            .add_row([1, 2, 3])
            .add_row([4, 5, 6])
            .add_row([])
            .add_row([7, 8, 9])
            .add_row([])
            .build();

        assert_eq!(array.get_row(0), Some(&[1, 2, 3][..]));
        assert_eq!(array.get_row(1), Some(&[4, 5, 6][..]));
        assert_eq!(array.get_row(2), Some(&[][..]));
        assert_eq!(array.get_row(3), Some(&[7, 8, 9][..]));
        assert_eq!(array.get_row(4), Some(&[][..]));
        assert_eq!(array.get_row(5), None);
    }

    #[test]
    fn test_iter_rows() {
        let array = Builder::<i64>::new()
            .add_row([])
            .add_row([1, 2, 3])
            .add_row([4, 5, 6])
            .add_row([])
            .add_row([7, 8, 9])
            .add_row([])
            .build();

        let mut iter = array.rows_iter();
        assert_eq!(iter.next(), Some(&[][..]));
        assert_eq!(iter.next(), Some(&[1, 2, 3][..]));
        assert_eq!(iter.next(), Some(&[4, 5, 6][..]));
        assert_eq!(iter.next(), Some(&[][..]));
        assert_eq!(iter.next(), Some(&[7, 8, 9][..]));
        assert_eq!(iter.next(), Some(&[][..]));
        assert_eq!(iter.next(), None);
    }
    #[test]
    fn test_get_rows() {
        let array = Builder::<i64>::new()
            .add_row([])
            .add_row([1, 2, 3])
            .add_row([4, 5, 6])
            .add_row([])
            .add_row([7, 8, 9])
            .add_row([])
            .build();
        println!("{array:?}");
        assert_eq!(array.get_rows(0..1), Some(&[][..]));
        assert_eq!(array.get_rows(0..2), Some(&[1, 2, 3][..]));
        assert_eq!(array.get_rows(2..2), Some(&[][..]));
        assert_eq!(array.get_rows(2..3), Some(&[4, 5, 6][..]));
        assert_eq!(array.get_rows(2..4), Some(&[4, 5, 6][..]));
        assert_eq!(array.get_rows(2..5), Some(&[4, 5, 6, 7, 8, 9][..]));
        assert_eq!(array.get_rows(..), Some(&[1, 2, 3, 4, 5, 6, 7, 8, 9][..]));
    }
}
