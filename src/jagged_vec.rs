//! An extensible (ie: can add more rows) [jagged array].
//!
//! [jagged array]: https://en.wikipedia.org/wiki/Jagged_array

use std::{fmt, marker::PhantomData, mem::ManuallyDrop};

use thiserror::Error;

/// [`JaggedVec::new`] construction error.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error {
    /// An `end` in `ends` was lower than a previous one.
    #[error(
        "Cannot build JaggedVec: `ends` represents the end of each row in `data`, \
        it should be monotonically increasing. \
        Found `end` at position {i} lower than `end` at position {}", .i - 1
    )]
    BadEnd { i: usize },
    /// An `end` in `ends` was too large.
    #[error(
        "Cannot build JaggedVec: `ends` represents the end of each row in `data`, \
        Yet, `end` at position {i} ({end}) is larger than the length of data ({len})"
    )]
    TooLongEnd { i: usize, len: u32, end: u32 },
}

/// A popped row from a [`JaggedVec`].
///
/// This implements `Deref[Mut]<Target = [T]>` meaning, you should be able to
/// use it whenever you can use a slice.
///
/// Note that this holds a reference to the `JaggedVec` it was created from,
/// meaning you cannot use any mutable or immutable methods on the parent
/// `JaggedVec` until you drop a `PoppedRow`, you may want to use the [`drop`]
/// std library function.
pub struct PoppedRow<'a, T> {
    array: ManuallyDrop<Box<[T]>>,
    lifetime: PhantomData<&'a ()>,
}
#[rustfmt::skip]
mod popped_row_impls {
    use super::PoppedRow;
    use std::ops::{Deref, DerefMut};
    use std::ptr;

    impl<'a, T> Deref for PoppedRow<'a, T> {
        type Target = [T];
        fn deref(&self) -> &Self::Target { self.as_ref() }
    }
    impl<'a, T> DerefMut for PoppedRow<'a, T>   { fn deref_mut(&mut self) -> &mut Self::Target { self.as_mut() } }
    impl<'a, T> AsRef<[T]> for PoppedRow<'a, T> { fn as_ref(&self)        -> &[T] { self.array.as_ref() } }
    impl<'a, T> AsMut<[T]> for PoppedRow<'a, T> { fn as_mut(&mut self)    -> &mut [T] { self.array.as_mut() } }
    impl<'a, T> Drop for PoppedRow<'a, T> {
        fn drop(&mut self) {
            let (ptr, len) = (self.array.as_mut_ptr(), self.array.len());
            let slice = ptr::slice_from_raw_parts_mut(ptr, len);
            unsafe { ptr::drop_in_place(slice) };
        }
    }
}

/// An extensible (ie: can add more rows) [jagged array].
///
/// **Note**: Unlike [`JaggedArray`](crate::JaggedArray), this implementation
/// can have 0 rows.
///
/// Refer to the `JaggedArray` "Design" section for more details.
///
/// [jagged array]: https://en.wikipedia.org/wiki/Jagged_array
#[derive(PartialEq, Eq, Clone)]
pub struct JaggedVec<T> {
    ends: Vec<u32>,
    data: Vec<T>,
    fully_popped: bool,
}
impl<T> Default for JaggedVec<T> {
    fn default() -> Self {
        Self::empty()
    }
}
impl<T> JaggedVec<T> where T: Clone {
    /// Add `row` at the end of the matrix from a slice. Each element of the slice will be cloned into the container.
    /// 
    /// # Example
    /// ```rust
    /// use datazoo::JaggedVec;
    ///
    /// let mut jagged = JaggedVec::empty();
    /// let mut source = vec![0, 1, 2, 3];
    /// jagged
    ///     .push_slice(&source)
    ///     .push_slice(&source[0..source.len() - 1]);
    /// assert_eq!(
    ///     jagged.into_vecs(),
    ///     vec![
    ///         vec![0, 1, 2, 3],
    ///         vec![0, 1, 2],
    ///     ],
    /// );
    /// ```
    pub fn push_slice(&mut self, slice: &[T]) -> &mut Self {
        if !self.fully_popped {
            self.ends.push(self.data.len() as u32);
        }
        self.data.extend_from_slice(slice);
        self.fully_popped = false;
        self
    }
}
impl<T> JaggedVec<T> {
    /// Add `row` at the end of the matrix.
    ///
    /// # Example
    /// ```rust
    /// use datazoo::JaggedVec;
    ///
    /// let mut jagged = JaggedVec::empty();
    /// jagged
    ///     .push_row([])
    ///     .push_row([0, 1, 2])
    ///     .push_row([3])
    ///     .push_row([7, 8])
    ///     .push_row([9])
    ///     .push_row([])
    ///     .push_row([11, 23]);
    /// assert_eq!(
    ///     jagged.into_vecs(),
    ///     vec![
    ///         vec![],
    ///         vec![0, 1, 2],
    ///         vec![3],
    ///         vec![7, 8],
    ///         vec![9],
    ///         vec![],
    ///         vec![11, 23],
    ///     ],
    /// );
    /// ```
    pub fn push_row(&mut self, row: impl IntoIterator<Item = T>) -> &mut Self {
        if !self.fully_popped {
            self.ends.push(self.data.len() as u32);
        }
        self.data.extend(row);
        self.fully_popped = false;
        self
    }
    /// Add an element to the last row, or create a first row if none exist yet.
    ///
    /// # Example
    /// ```rust
    /// use datazoo::JaggedVec;
    ///
    /// let mut jagged = JaggedVec::empty();
    /// jagged.push_row([0, 1, 2]).push_row([3]);
    /// jagged.push(4);
    /// assert_eq!(jagged.into_vecs(), vec![vec![0, 1, 2], vec![3, 4]]);
    /// ```
    pub fn push(&mut self, elem: T) {
        self.fully_popped = false;
        self.data.push(elem);
    }
    /// Add multiple elements to the last row.
    ///
    /// # Example
    /// ```rust
    /// use datazoo::JaggedVec;
    ///
    /// let mut jagged = JaggedVec::empty();
    /// jagged.push_row([0, 1, 2]).push_row([3]);
    /// jagged.extend_last_row([4, 5, 6]);
    /// assert_eq!(jagged.into_vecs(), vec![vec![0, 1, 2], vec![3, 4, 5, 6]]);
    /// ```
    pub fn extend_last_row(&mut self, elems: impl IntoIterator<Item = T>) {
        self.fully_popped = false;
        self.data.extend(elems);
    }
    /// Remove all rows from this `JaggedVec`.
    pub fn clear(&mut self) {
        self.fully_popped = true;
        self.data.clear();
        self.ends.clear();
    }

    // TODO(feat): pop_elem. But make sure we aren't removing from non-last row
    // in case last row is empty.

    /// Remove the last row from the matrix, returning it.
    ///
    /// Note that the returned value holds a reference to the jagged vec, which
    /// will prevent using this `JaggedVec` until the returned [`PoppedRow`] is dropped.
    ///
    /// # Example
    /// ```rust
    /// use datazoo::JaggedVec;
    ///
    /// let mut jagged = JaggedVec::empty();
    /// jagged.push_row([0, 1, 2]).push_row([3]).push_row([4, 5, 6, 7]);
    /// let popped = jagged.pop_row();
    /// assert_eq!(popped.as_deref(), Some(&[4, 5, 6, 7][..]));
    ///
    /// drop(popped); // need to drop `popped`, otherwise can't access `jagged`.
    ///
    /// assert_eq!(jagged.get_row(2), None);
    /// assert_eq!(jagged.clone().into_vecs(), vec![vec![0, 1, 2], vec![3]]);
    /// jagged.pop_row();
    /// jagged.pop_row();
    /// assert_eq!(jagged.height(), 0);
    /// ```
    pub fn pop_row(&mut self) -> Option<PoppedRow<T>> {
        if self.fully_popped {
            return None;
        }
        self.fully_popped = self.ends.is_empty();
        let last_end = self.ends.pop().unwrap_or(0) as usize;
        let last_len = self.data.len();
        let popped_len = last_len - last_end;

        // SAFETY: by construction, `last_end` is always equal or smaller
        // than `len`, which itself is always smaller than capacity.
        unsafe { self.data.set_len(last_end) };
        let popped_row = unsafe {
            let popped_ptr = self.data.as_mut_ptr().add(last_end);
            Vec::from_raw_parts(popped_ptr, popped_len, popped_len)
        };
        Some(PoppedRow {
            array: ManuallyDrop::new(popped_row.into_boxed_slice()),
            lifetime: PhantomData,
        })
    }
    /// How many cells are contained in this `JaggedVec`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }
    /// Is this vector empty (no cells, may have several empty rows).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    /// How many rows this `JaggedVec` has.
    #[must_use]
    pub fn height(&self) -> usize {
        if self.fully_popped {
            0
        } else {
            self.ends.len() + 1
        }
    }
    /// The empty `JaggedVec`, identical to `JaggedVec::default()`.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            data: Vec::new(),
            ends: Vec::new(),
            fully_popped: true,
        }
    }
    /// Create a [`JaggedVec`] of `ends.len() + 1` rows, values of `ends` are the
    /// end indicies (exclusive) of each row in `data`.
    ///
    /// Note that the _last index_ should be elided.
    /// The last row will be the values between the last `end` in `ends` and
    /// the total size of the `data` array.
    ///
    /// # Errors
    /// The `ends` slice is invalid:
    /// - An `ends[i] > data.len()`
    /// - An `ends[i+1] < ends[i]`
    ///
    /// # Example
    /// ```rust
    /// use datazoo::JaggedVec;
    ///
    /// let ends = [0, 0, 3, 4, 7, 9, 10, 10]; // len = 8
    /// let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 23];
    /// let jagged = JaggedVec::new(ends.to_vec(), data.to_vec()).unwrap();
    /// let iliffe = jagged.into_vecs();
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
    ///         vec![11, 23],
    ///     ], // len = 9
    /// );
    /// ```
    pub fn new(ends: Vec<u32>, data: Vec<T>) -> Result<Self, Error> {
        let mut previous_end = 0;
        let last_end = data.len() as u32;
        for (i, end) in ends.iter().enumerate() {
            if *end > last_end {
                return Err(Error::TooLongEnd { i, len: last_end, end: *end });
            }
            if *end < previous_end {
                return Err(Error::BadEnd { i });
            }
            previous_end = *end;
        }
        Ok(Self { ends, data, fully_popped: false })
    }
    /// Get slice to row at given `index`.
    ///
    /// # Panics
    /// When `index > self.height()`.
    #[inline]
    #[must_use]
    pub fn row(&self, index: usize) -> &[T] {
        self.get_row(index).unwrap()
    }
    /// Get slice to row at given `index`.
    ///
    /// Returns `None` when `index > self.height()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use datazoo::JaggedVec;
    ///
    /// let ends = [0, 0, 3, 4, 7, 9, 10, 10];
    /// let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    /// let jagged = JaggedVec::new(ends.to_vec(), data.to_vec()).unwrap();
    ///
    /// assert_eq!(jagged.get_row(4), Some(&[4, 5, 6][..]));
    /// ```
    #[inline]
    #[must_use]
    pub fn get_row(&self, index: usize) -> Option<&[T]> {
        if index > self.ends.len() {
            return None;
        }
        // TODO(perf): verify generated code elides bound checks.
        let get_end = |end: &u32| *end as usize;

        let start = index.checked_sub(1).map_or(0, |i| self.ends[i]) as usize;
        let end = self.ends.get(index).map_or(self.data.len(), get_end);
        // SAFETY: We always push ends that are smaller that data.len() to self.end
        Some(unsafe { self.data.get_unchecked(start..end) })
    }
    /// Get `V` at exact `direct_index` ignoring row sizes,
    /// acts as if the whole array was a single row.
    ///
    /// `None` when `direct_index` is out of bound.
    ///
    /// # Example
    ///
    /// ```rust
    /// use datazoo::JaggedVec;
    ///
    /// let ends = [0, 0, 3, 4, 7, 9, 10, 10];
    /// let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    /// let jagged = JaggedVec::new(ends.to_vec(), data.to_vec()).unwrap();
    ///
    /// assert_eq!(jagged.get(4), Some(&4));
    /// ```
    #[inline]
    #[must_use]
    pub fn get(&self, direct_index: usize) -> Option<&T> {
        self.data.get(direct_index)
    }
    /// Turn this compact jagged array into a sparse representation.
    ///
    /// The returned `Vec<Vec<V>>` is an [Iliffe vector]. Iterating over it will
    /// be much slower than iterating over `JaggedVec`, but extending individual
    /// rows is much less costly.
    ///
    /// [Iliffe vector]: https://en.wikipedia.org/wiki/Iliffe_vector
    #[must_use]
    pub fn into_vecs(self) -> Vec<Vec<T>> {
        let Self { ends, mut data, fully_popped } = self;
        if fully_popped {
            return Vec::new();
        }
        let mut iliffe = Vec::with_capacity(ends.len() + 1);
        let mut last_end = 0;

        // TODO(perf): this is slow as heck because each drain needs to move
        // forward the end of the `data` vec, if we reverse ends here, we can
        // skip the nonsense.
        for end in ends {
            let size = (end - last_end) as usize;
            iliffe.push(data.drain(..size).collect());
            last_end = end;
        }
        // the last row.
        iliffe.push(data);
        iliffe
    }
    /// Iterate over all the rows in the `JaggedVec`.
    pub fn rows(&self) -> impl Iterator<Item = &[T]> {
        (0..self.height()).map(|i| unsafe { self.get_row(i).unwrap_unchecked() })
    }
}
impl<T: fmt::Debug> fmt::Debug for JaggedVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for row in self.rows() {
            list.entry(&row);
        }
        list.finish()
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use std::sync::atomic::{AtomicI64, Ordering};

    struct RefCount<'a>(&'a AtomicI64);
    impl<'a> RefCount<'a> {
        fn new(atomic: &'a AtomicI64) -> Self {
            atomic.fetch_add(1, Ordering::Relaxed);
            Self(atomic)
        }
    }
    impl Drop for RefCount<'_> {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn count_drops() {
        let count = AtomicI64::new(0);
        let mk_ref = || RefCount::new(&count);
        let mut jagged = JaggedVec::empty();
        jagged
            .push_row([mk_ref(), mk_ref()])
            .push_row([mk_ref(), mk_ref(), mk_ref(), mk_ref()])
            .push_row([mk_ref()]);
        assert_eq!(count.load(Ordering::Relaxed), 7);
        let popped = jagged.pop_row().unwrap();
        assert_eq!(count.load(Ordering::Relaxed), 7);
        drop(popped);
        assert_eq!(count.load(Ordering::Relaxed), 6);
        jagged.pop_row();
        assert_eq!(count.load(Ordering::Relaxed), 2);
        drop(jagged);
        assert_eq!(count.load(Ordering::Relaxed), 0);
    }
}
