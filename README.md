A 3-pointer iterator that moves out of a `Vec<T>` or `Box<[T]>`

## Why?

If you want to iterate and move items out of a `Box<[T]>`, you'd normally use
the
[`vec::IntoIter`](https://doc.rust-lang.org/stable/std/vec/struct.IntoIter.html)
iterator. (Note: The [upcoming](https://github.com/rust-lang/rust/pull/124097)
`IntoIterator` impl for `Box<[T]>` also uses `vec::IntoIter`.) This is fine for
most use cases.

However, storing a large collection of `vec::IntoIter` iterators is suboptimal.
This is because `vec::IntoIter` is represented as 4 pointers, which is one more
than strictly necessary.

This crate provides an `IntoSmallIter` type, which is represented as 3 pointers.
In exchange for this smaller size, this type doesn't implement
`DoubleEndedIterator`

## Usage

TODO