use criterion::{Criterion, criterion_group, criterion_main};
use forge_json_repair::json_repair;
use serde_json::Value;

fn bench_repair_truncated(c: &mut Criterion) {
    // Realistic: a truncated LLM JSON output missing closing braces.
    let truncated = r#"{"name": "Alice", "age": 30, "tags": ["rust", "perf"#;

    c.bench_function("json_repair/truncated_object", |b| {
        b.iter(|| {
            let _: Value = json_repair(std::hint::black_box(truncated)).unwrap_or_default();
        });
    });
}

fn bench_repair_valid(c: &mut Criterion) {
    // Already-valid JSON — repair should be a cheap pass-through.
    let valid = r#"{"name":"Bob","scores":[1,2,3,4,5],"active":true}"#;

    c.bench_function("json_repair/already_valid", |b| {
        b.iter(|| {
            let _: Value = json_repair(std::hint::black_box(valid)).unwrap_or_default();
        });
    });
}

criterion_group!(benches, bench_repair_truncated, bench_repair_valid);
criterion_main!(benches);
