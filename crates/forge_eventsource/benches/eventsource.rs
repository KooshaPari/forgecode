//! Bench the SSE event-stream parser using forge_eventsource_stream directly,
//! since forge_eventsource wraps it over reqwest (needs a live server).

use criterion::{Criterion, criterion_group, criterion_main};
use forge_eventsource_stream::EventStream;
use futures::StreamExt;

fn bench_parse_sse_events(c: &mut Criterion) {
    // Simulate a batch of SSE lines as a stream of byte chunks.
    let events = concat!(
        "data: {\"type\":\"content\",\"delta\":\"Hello\"}\n\n",
        "data: {\"type\":\"content\",\"delta\":\" world\"}\n\n",
        "data: {\"type\":\"content\",\"delta\":\"!\"}\n\n",
        "data: [DONE]\n\n",
    );

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    c.bench_function("eventsource/parse_4_events", |b| {
        b.iter(|| {
            rt.block_on(async {
                let bytes_stream = futures::stream::iter(
                    std::hint::black_box(events)
                        .as_bytes()
                        .chunks(32)
                        .map(|c| {
                            Ok::<_, std::convert::Infallible>(bytes::Bytes::copy_from_slice(c))
                        })
                        .collect::<Vec<_>>(),
                );
                let mut stream = EventStream::new(bytes_stream);
                let mut count = 0usize;
                while stream.next().await.is_some() {
                    count += 1;
                }
                count
            })
        });
    });
}

criterion_group!(benches, bench_parse_sse_events);
criterion_main!(benches);
