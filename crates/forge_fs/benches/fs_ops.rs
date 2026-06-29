use criterion::{Criterion, criterion_group, criterion_main};
use forge_fs::ForgeFS;
use std::io::Write;
use tempfile::NamedTempFile;

fn bench_read_utf8(c: &mut Criterion) {
    // Write a small file once; read it in the hot loop.
    let mut f = NamedTempFile::new().expect("tempfile");
    let content = "forge fs benchmark\n".repeat(100); // ~1.9 KB
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    let path = f.path().to_path_buf();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    c.bench_function("fs/read_utf8_2kb", |b| {
        b.iter(|| {
            rt.block_on(async {
                ForgeFS::read_utf8(std::hint::black_box(&path))
                    .await
                    .expect("read ok")
            })
        });
    });
}

criterion_group!(benches, bench_read_utf8);
criterion_main!(benches);
