use criterion::{Criterion, criterion_group, criterion_main};
use thriftp_core::model::ProtocolCapabilities;
use thriftp_protocols::parse_mlsd_line;

fn parse_mlsd(c: &mut Criterion) {
    let line = "type=file;size=1024;modify=20260101000000;perm=adfr; report.pdf";
    c.bench_function("parse_mlsd_line", |b| {
        b.iter(|| parse_mlsd_line(line, "/remote", ProtocolCapabilities::default()).unwrap())
    });
}

criterion_group!(benches, parse_mlsd);
criterion_main!(benches);
