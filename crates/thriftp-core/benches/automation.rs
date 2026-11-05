//! Benchmarks for scope canonicalization, authorization, audit, and control parsing.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use thriftp_core::automation::{canonical_remote, path_in_scope};
use thriftp_core::host::protocol::RpcRequest;

fn scope(c: &mut Criterion) {
    c.bench_function("canonical_remote", |b| {
        b.iter(|| canonical_remote(black_box("/srv/app/releases/2026/file.bin")).unwrap());
    });
    c.bench_function("path_in_scope", |b| {
        b.iter(|| path_in_scope(black_box("/srv/app/releases/file"), black_box("/srv/app")));
    });
}

fn authorization(c: &mut Criterion) {
    let grant_operations = ["browse.list", "transfers.upload", "files.delete"];
    c.bench_function("grant_contains", |b| {
        b.iter(|| {
            grant_operations
                .iter()
                .any(|operation| *operation == black_box("transfers.upload"))
        });
    });
}

fn control(c: &mut Criterion) {
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"transfers.status","auth":"token","params":{"transferId":"0f8fad5b-d9cb-469f-a165-70867728950e"}}"#;
    c.bench_function("rpc_parse", |b| {
        b.iter(|| RpcRequest::parse(black_box(request)).unwrap());
    });
}

fn audit(c: &mut Criterion) {
    use sha2::{Digest, Sha256};
    c.bench_function("audit_hash", |b| {
        b.iter(|| {
            let digest = Sha256::digest(black_box(b"{\"operation\":\"files.delete\"}"));
            black_box(digest);
        });
    });
}

criterion_group!(benches, scope, authorization, control, audit);
criterion_main!(benches);
