use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use small_iter::{IntoSmallIterExt, SmallIter};
use std::hint::black_box;
use std::{iter, vec};
use thin_vec::ThinVec;

const NUM_ITERS: usize = 100_000;
const NUM_ELEMENTS: usize = 100;

fn consume<I: Iterator<Item = u8>>(mut iters: Vec<I>) {
    for _ in 0..NUM_ELEMENTS {
        for iter in &mut iters {
            black_box(iter.next());
        }
    }
}

fn using_small_iter() {
    let iters: Vec<SmallIter<u8>> = iter::repeat_with(|| {
        (0..(NUM_ELEMENTS as u8))
            .collect::<Vec<u8>>()
            .into_small_iter()
    })
    .take(NUM_ITERS)
    .collect();
    consume(black_box(iters));
}

fn using_thin_vec_into_iter() {
    let iters: Vec<thin_vec::IntoIter<u8>> = iter::repeat_with(|| {
        (0..(NUM_ELEMENTS as u8))
            .collect::<ThinVec<u8>>()
            .into_iter()
    })
    .take(NUM_ITERS)
    .collect();
    consume(black_box(iters));
}

fn using_vec_into_iter() {
    let iters: Vec<vec::IntoIter<u8>> =
        iter::repeat_with(|| (0..(NUM_ELEMENTS as u8)).collect::<Vec<u8>>().into_iter())
            .take(NUM_ITERS)
            .collect();
    consume(black_box(iters));
}

fn bench_vec_of_iters(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec_of_iters");
    group.bench_function(BenchmarkId::new("using_small_iter", ""), |b| {
        b.iter(using_small_iter)
    });
    group.bench_function(BenchmarkId::new("using_thin_vec_into_iter", ""), |b| {
        b.iter(using_thin_vec_into_iter)
    });
    group.bench_function(BenchmarkId::new("using_vec_into_iter", ""), |b| {
        b.iter(using_vec_into_iter)
    });
    group.finish();
}

criterion_group!(benches, bench_vec_of_iters);
criterion_main!(benches);
