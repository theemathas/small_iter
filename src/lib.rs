use std::{marker::PhantomData, ptr::NonNull};

/// An iterator that moves out of a boxed slice.
///
/// This struct is created by TODO
///
/// Unlike [`std::vec::IntoIter`], which is represented as 4 pointers,
/// this iterator is represented as 3 pointers.
/// In exchange, it does not implement [`DoubleEndedIterator`](std::iter::DoubleEndedIterator).
pub struct IntoSmallIter<T> {
    /*
    Similarly to how `std::vec::IntoIter` is implemented,
    we store things differently depending on whether
    `T` is a ZST or not.

    If `T` is not a ZST:
    - The allocation is `allocation_start..end`
    - The remaining elements are at `elements_start..end`

    If `T` is a ZST:
    - `allocation_start == elements_start == dangling`
    - `end` is n bytes after `dangling`, where n is the number of elements
     */
    allocation_start: NonNull<T>,
    elements_start: NonNull<T>,
    end: *const T,
    _phantom: PhantomData<T>,
}
