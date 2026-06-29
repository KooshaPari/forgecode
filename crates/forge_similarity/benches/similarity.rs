use criterion::{Criterion, criterion_group, criterion_main};
use forge_similarity::config::Tier;
use forge_similarity::select_provider;

fn bench_hash_only_compare(c: &mut Criterion) {
    let provider = select_provider(&Tier::T0, None);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    c.bench_function("similarity/hash_only_compare", |b| {
        b.iter(|| {
            rt.block_on(async {
                provider
                    .compare(
                        std::hint::black_box("agent-42"),
                        std::hint::black_box(
                            "Write a Rust function that sorts a Vec<String> alphabetically.",
                        ),
                    )
                    .await
                    .expect("compare ok")
            })
        });
    });
}

criterion_group!(benches, bench_hash_only_compare);
criterion_main!(benches);
