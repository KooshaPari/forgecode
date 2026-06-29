use criterion::{Criterion, criterion_group, criterion_main};
use forge_stream::MpscStream;
use futures::StreamExt;

fn bench_mpsc_stream_throughput(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    // Measure round-trip latency for sending N items through MpscStream.
    const N: usize = 64;

    c.bench_function("stream/mpsc_64_items", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut stream = MpscStream::spawn(|tx| async move {
                    for i in 0u64..N as u64 {
                        let _ = tx.send(i).await;
                    }
                });
                let mut count = 0usize;
                while stream.next().await.is_some() {
                    count += 1;
                }
                count
            })
        });
    });
}

criterion_group!(benches, bench_mpsc_stream_throughput);
criterion_main!(benches);
