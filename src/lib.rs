//! TODO write crate-level docs

use core::slice;
use std::{
    iter::FusedIterator,
    marker::PhantomData,
    mem::{size_of, ManuallyDrop},
    ptr::{self, NonNull},
};

trait Sealed {}

/// An extension trait that provides the `into_small_iter` method on boxed
/// slices.
#[allow(private_bounds)]
pub trait BoxedSliceExt: Sealed {
    /// The type of the elements in the boxed slice.
    type Item;

    /// Consumes the boxed slice and returns an [`IntoSmallIter`] that moves out
    /// of it.
    fn into_small_iter(self) -> IntoSmallIter<Self::Item>;
}

impl<T> Sealed for Box<[T]> {}

impl<T> BoxedSliceExt for Box<[T]> {
    type Item = T;

    fn into_small_iter(self) -> IntoSmallIter<T> {
        // SAFETY: the slice is owned by `self`, so it's safe to move out of it.
        let slice_ptr: *mut [T] = Box::into_raw(self);
        let (start, end) = if const { size_of::<T>() == 0 } {
            let dangling = NonNull::<T>::dangling();
            (
                dangling,
                dangling.as_ptr().wrapping_byte_add(slice_ptr.len()),
            )
        } else {
            let first_element_ptr = slice_ptr.cast::<T>();
            // SAFETY: We set `start` and `end` to be the beginning and end of the slice.
            // The elements in between are initialized.
            unsafe {
                (
                    NonNull::new_unchecked(first_element_ptr),
                    first_element_ptr.add(slice_ptr.len()),
                )
            }
        };
        IntoSmallIter {
            elements_start: start,
            allocation_start: start,
            end,
            _phantom: PhantomData,
        }
    }
}

/// An iterator that moves out of a boxed slice.
///
/// This struct is created by [`BoxedSliceExt::into_small_iter`]
///
/// Unlike [`std::vec::IntoIter`], which is represented as 4 pointers,
/// this iterator is represented as 3 pointers.
/// In exchange, it does not implement [`DoubleEndedIterator`].
#[derive(Debug)]
pub struct IntoSmallIter<T> {
    /*
    Similarly to how `std::vec::IntoIter` is implemented,
    we store things differently depending on whether
    `T` is a ZST or not.

    If `T` is not a ZST:
    - The allocation is `allocation_start..end`
    - The remaining elements are at `elements_start..end`
    - SAFETY invariant: the memory from `elements_start` to `end` is initialized

    If `T` is a ZST:
    - `allocation_start == elements_start == dangling`
    - `end` is n bytes after `dangling`, where n is the number of elements
     */
    elements_start: NonNull<T>,
    allocation_start: NonNull<T>,
    end: *const T,
    _phantom: PhantomData<T>,
}

impl<T> IntoSmallIter<T> {
    /// Returns the remaining elements in the iterator as a slice.
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.elements_start.as_ptr(), self.elements_len()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.elements_start.as_ptr(), self.elements_len()) }
    }

    /// Returns the number of elements remaining in the iterator.
    fn elements_len(&self) -> usize {
        if const { size_of::<T>() == 0 } {
            (self.end as usize).wrapping_sub(self.elements_start.as_ptr() as usize)
        } else {
            // SAFETY: `elements_start..end` is from the same allocation.
            unsafe { self.end.offset_from(self.elements_start.as_ptr()) as usize }
        }
    }

    /// Returns the number of elements in the allocation, including
    /// uninitialized elements.
    fn allocation_len(&self) -> usize {
        if const { size_of::<T>() == 0 } {
            0
        } else {
            // SAFETY: `allocation_start..end` is from the same allocation.
            unsafe { self.end.offset_from(self.allocation_start.as_ptr()) as usize }
        }
    }
}

unsafe impl<T: Send> Send for IntoSmallIter<T> {}
unsafe impl<T: Sync> Sync for IntoSmallIter<T> {}

impl<T> Iterator for IntoSmallIter<T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if ptr::eq(self.elements_start.as_ptr(), self.end) {
            None
        } else if const { size_of::<T>() == 0 } {
            self.end = self.end.wrapping_byte_sub(1);
            // SAFETY: `T` is a ZST, so we can conjure one from thin air.
            Some(unsafe { NonNull::<T>::dangling().as_ptr().read() })
        } else {
            // SAFETY: the memory is initialized as per the invariant.
            let element = unsafe { self.elements_start.as_ptr().read() };
            // SAFETY: `elements_start..end` is from the same allocation, and
            // we've checked that we're not at the end, so we can advance by 1.
            self.elements_start =
                unsafe { NonNull::new_unchecked(self.elements_start.as_ptr().add(1)) };
            Some(element)
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.elements_len();
        (len, Some(len))
    }

    #[inline]
    fn count(self) -> usize {
        self.elements_len()
    }
}

impl<T> ExactSizeIterator for IntoSmallIter<T> {}

impl<T> FusedIterator for IntoSmallIter<T> {}

impl<T> AsRef<[T]> for IntoSmallIter<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> AsMut<[T]> for IntoSmallIter<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> Default for IntoSmallIter<T> {
    fn default() -> Self {
        <Box<[T]>>::default().into_small_iter()
    }
}

impl<T: Clone> Clone for IntoSmallIter<T> {
    fn clone(&self) -> Self {
        <Box<[T]>>::from(self.as_slice()).into_small_iter()
    }
}

impl<T> Drop for IntoSmallIter<T> {
    fn drop(&mut self) {
        struct DropGuard<'a, T>(&'a mut IntoSmallIter<T>);

        impl<T> Drop for DropGuard<'_, T> {
            // Drop the Box allocation, but not the contained elements in the slice.
            fn drop(&mut self) {
                let slice_ptr: *mut [ManuallyDrop<T>] = ptr::slice_from_raw_parts_mut(
                    self.0.allocation_start.as_ptr().cast(),
                    self.0.allocation_len(),
                );
                // SAFETY: We reconstruct the original `Box<[T]>`, but as a
                // `Box<[ManuallyDrop<T>]>`, and then drop it.
                unsafe { drop(Box::from_raw(slice_ptr)) };
            }
        }

        let guard = DropGuard(self);
        // SAFETY: We drop only the initialized elements.
        unsafe {
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(
                guard.0.elements_start.as_ptr(),
                guard.0.elements_len(),
            ));
        }
        // guard is dropped here
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_exhaust() {
        let s: Box<[Box<i32>]> = Box::new([Box::new(1), Box::new(2), Box::new(3)]);
        let mut iter = s.into_small_iter();
        assert_eq!(iter.size_hint(), (3, Some(3)));
        assert_eq!(iter.as_slice(), &[Box::new(1), Box::new(2), Box::new(3)]);
        assert_eq!(iter.next(), Some(Box::new(1)));
        assert_eq!(iter.size_hint(), (2, Some(2)));
        assert_eq!(iter.as_slice(), &[Box::new(2), Box::new(3)]);
        assert_eq!(iter.next(), Some(Box::new(2)));
        assert_eq!(iter.size_hint(), (1, Some(1)));
        assert_eq!(iter.as_slice(), &[Box::new(3)]);
        assert_eq!(iter.next(), Some(Box::new(3)));
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.as_slice(), &[]);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.as_slice(), &[]);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.as_slice(), &[]);
    }

    #[test]
    fn basic_partial() {
        let s: Box<[Box<i32>]> = Box::new([Box::new(1), Box::new(2), Box::new(3)]);
        let mut iter = s.into_small_iter();
        assert_eq!(iter.next(), Some(Box::new(1)));
        assert_eq!(iter.next(), Some(Box::new(2)));
        // Drop the iterator here
    }

    #[test]
    fn basic_exhaust_zst() {
        let s: Box<[()]> = Box::new([(); 3]);
        let mut iter = s.into_small_iter();
        assert_eq!(iter.size_hint(), (3, Some(3)));
        assert_eq!(iter.as_slice(), &[(), (), ()]);
        assert_eq!(iter.next(), Some(()));
        assert_eq!(iter.size_hint(), (2, Some(2)));
        assert_eq!(iter.as_slice(), &[(), ()]);
        assert_eq!(iter.next(), Some(()));
        assert_eq!(iter.size_hint(), (1, Some(1)));
        assert_eq!(iter.as_slice(), &[()]);
        assert_eq!(iter.next(), Some(()));
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.as_slice(), &[]);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.as_slice(), &[]);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.as_slice(), &[]);
    }

    #[test]
    fn basic_partial_zst() {
        let s: Box<[()]> = Box::new([(); 3]);
        let mut iter = s.into_small_iter();
        assert_eq!(iter.next(), Some(()));
        assert_eq!(iter.next(), Some(()));
        // Drop the iterator here
    }
}
