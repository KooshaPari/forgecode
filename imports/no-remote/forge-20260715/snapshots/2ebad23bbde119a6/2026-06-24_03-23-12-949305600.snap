use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn benchmark(c: &mut Criterion) {
    c.bench_function("noop", |b| b.iter(|| black_box(42)));
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
