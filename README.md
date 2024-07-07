A 3-pointer iterator that moves out of a `Vec<T>` or `Box<[T]>`

[Documentation link](https://docs.rs/small_iter)

## Why?

If you want to iterate and move items out of a `Vec<T>`, you'd normally call
`.into_iter()`, producing a `vec::IntoIter` iterator. (Note: The
[upcoming](https://github.com/rust-lang/rust/pull/124097) `IntoIterator` impl
for `Box<[T]>` also uses `vec::IntoIter`.) This is fine for most use cases.

However, storing a large collection of `vec::IntoIter` iterators might be
suboptimal for memory usage. This is because `vec::IntoIter` is represented as 4
pointers, which is one more than strictly necessary if all you want is iterating
in one direction.

This crate provides a `SmallIter` type, which is represented as 3 pointers. In
exchange for this smaller size, this type doesn't implement
`DoubleEndedIterator`.

## Usage

The `IntoSmallIterExt` trait provides the `into_small_iter()` method, which
allows you to produce `SmallIter` iterators from a `Vec<T>` or a `Box<[T]>`.

```rust
use small_iter::IntoSmallIterExt;

let v = vec![1, 2, 3];
let iter = v.into_small_iter();
let v2: Vec<_> = iter.collect();
assert_eq!(v2, vec![1, 2, 3]);
```

The benefits of the space savings of this crate is most likely to be relevant if
you store a bunch of iterators.

```rust
use small_iter::{IntoSmallIterExt, SmallIter};

let v = vec![vec![1, 2], vec![3, 4], vec![5, 6]];
let mut iters: Vec<SmallIter<i32>> = v.into_iter().map(|v| v.into_small_iter()).collect();
assert_eq!(iters[0].next(), Some(1));
assert_eq!(iters[1].next(), Some(3));
assert_eq!(iters[2].next(), Some(5));
assert_eq!(iters[0].next(), Some(2));
assert_eq!(iters[1].next(), Some(4));
assert_eq!(iters[2].next(), Some(6));
```

### Caveat

For `Vec<T>`, if there is excess capacity in the vector, calling
`into_small_iter` will first shrink the allocation to fit the existing elements.
Depending on the allocator, this may reallocate.

On the other hand, calling `into_small_iter` on a `Box<[T]>` is cheap.

## Benchmark results

I have benchmarked (on a Macbook Pro 2021) the following workload (which is the
kind of workload that this crate is intended for): Construct 100,000 iterators,
each containing 100 `u8`s. Then, get the first element of each iterator, then
the second, and so on.

This workload is performed in three ways:
* using `SmallIter` (this crate)
  * taking 20.4ms on average
* using `std::vec::IntoIter`
  * taking 30.5ms on average
* using `thin_vec::IntoIter` (from the `thin-vec` crate)
  * taking 21.9ms on average

The source code for the benchmark can be found
[here](https://github.com/theemathas/small_iter/tree/master/benches/vec_of_iters.rs).

![Violin plot of the running times](https://raw.githubusercontent.com/theemathas/small_iter/master/violin.svg)