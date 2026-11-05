use criterion::{Criterion, criterion_group, criterion_main};
use thriftp_core::browser::listing::{ListingSession, stream_local};
use thriftp_core::browser::metrics::measure_local;

fn listing_benchmark(c: &mut Criterion) {
    let directory = tempfile::tempdir().unwrap();
    for index in 0..1_000 {
        std::fs::write(directory.path().join(format!("file-{index:04}")), b"x").unwrap();
    }

    c.bench_function("listing_1000_entries", |b| {
        b.iter(|| measure_local(|| thriftp_core::browser::list_local(directory.path())).unwrap())
    });

    c.bench_function("streaming_batches_1000_entries", |b| {
        b.iter(|| {
            let session = ListingSession::new();
            let mut batches = 0;
            stream_local(directory.path(), 100, &session, |_| batches += 1).unwrap();
            batches
        })
    });
}

criterion_group!(benches, listing_benchmark);
criterion_main!(benches);
