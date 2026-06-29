use criterion::{Criterion, criterion_group, criterion_main};
use forge_drift::index::DriftIndex;

fn bench_drift_index_observe_jaccard(c: &mut Criterion) {
    let index = DriftIndex::new();
    let agent = "bench-agent";
    // Seed the index with a baseline prompt.
    index.observe(
        agent,
        "Write a Rust function that sorts a Vec<String> alphabetically.",
    );

    let probe = "Write a Rust function that sorts a Vec<i32> numerically.";

    c.bench_function("drift/observe_then_jaccard", |b| {
        b.iter(|| {
            index.observe(std::hint::black_box(agent), std::hint::black_box(probe));
            index.jaccard(std::hint::black_box(agent), std::hint::black_box(probe))
        });
    });
}

criterion_group!(benches, bench_drift_index_observe_jaccard);
criterion_main!(benches);
