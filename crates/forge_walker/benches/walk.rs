use criterion::{Criterion, criterion_group, criterion_main};
use forge_walker::Walker;
use std::fs;
use tempfile::TempDir;

fn bench_walk_small_tree(c: &mut Criterion) {
    // Build a small deterministic directory tree once outside the timed loop.
    let dir = TempDir::new().expect("tempdir");
    for i in 0..20 {
        let sub = dir.path().join(format!("sub{i}"));
        fs::create_dir_all(&sub).unwrap();
        for j in 0..5 {
            fs::write(sub.join(format!("file{j}.txt")), b"hello criterion").unwrap();
        }
    }
    // 20 subdirs × 5 files = 100 files total.

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    c.bench_function("walker/small_tree_100_files", |b| {
        b.iter(|| {
            rt.block_on(async {
                Walker::max_all()
                    .cwd(dir.path().to_path_buf())
                    .get()
                    .await
                    .expect("walk ok")
            })
        });
    });
}

criterion_group!(benches, bench_walk_small_tree);
criterion_main!(benches);
