//! Benchmark for MCP tool dispatch.

use criterion::{Criterion, criterion_group, criterion_main};
use serde_json::json;
use std::hint::black_box;
use thriftp_mcp::server::dispatch_for_bench;

fn dispatch(c: &mut Criterion) {
    c.bench_function("mcp_tools_list", |b| {
        b.iter(|| {
            let _ = dispatch_for_bench(black_box("tools/list"), black_box(&json!({})));
        });
    });
    c.bench_function("mcp_discover", |b| {
        b.iter(|| {
            let _ = dispatch_for_bench(black_box("server/discover"), black_box(&json!({})));
        });
    });
}

criterion_group!(benches, dispatch);
criterion_main!(benches);
