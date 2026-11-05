//! Benchmark for CLI result-envelope serialization.

use criterion::{Criterion, criterion_group, criterion_main};
use serde_json::json;
use std::hint::black_box;

fn envelope(c: &mut Criterion) {
    let value = json!({
        "schemaVersion": 1,
        "command": "transfer.list",
        "ok": true,
        "exitCode": 0,
        "data": {"transfers": [{"id": "0f8fad5b-d9cb-469f-a165-70867728950e", "state": "completed"}]},
        "warnings": [],
        "error": null,
        "meta": {
            "cliVersion": "0.1.0",
            "automationContractVersion": "1.0.0",
            "mcpProtocolVersions": "2026-07-28, 2025-11-25",
            "hostMode": "local"
        }
    });
    c.bench_function("cli_json_output", |b| {
        b.iter(|| serde_json::to_string_pretty(black_box(&value)).unwrap());
    });
}

criterion_group!(benches, envelope);
criterion_main!(benches);
