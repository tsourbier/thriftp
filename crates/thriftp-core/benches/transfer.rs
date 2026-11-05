use criterion::{Criterion, criterion_group, criterion_main};
use thriftp_core::transfers::stream::copy_chunked;
use tokio::io::duplex;

fn transfer_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    c.bench_function("copy_1mb_in_64k_chunks", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let (mut writer, mut reader) = duplex(128 * 1024);
                let producer = tokio::spawn(async move {
                    use tokio::io::AsyncWriteExt;
                    let chunk = vec![1_u8; 64 * 1024];
                    for _ in 0..16 {
                        writer.write_all(&chunk).await.unwrap();
                    }
                });
                let mut sink = Vec::new();
                let stats = copy_chunked(&mut reader, &mut sink, 64 * 1024, |_| true)
                    .await
                    .unwrap();
                producer.abort();
                stats.bytes
            })
        })
    });
}

criterion_group!(benches, transfer_benchmark);
criterion_main!(benches);
