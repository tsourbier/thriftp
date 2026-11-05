use criterion::{Criterion, criterion_group, criterion_main};
use thriftp_core::model::{AuthMethod, ConnectionProfile, Protocol};
use thriftp_core::sorting::sort_profiles;

fn profile(name: &str) -> ConnectionProfile {
    ConnectionProfile::new(
        name,
        Protocol::Sftp,
        "example.test",
        22,
        "tester",
        AuthMethod::Password,
        30,
    )
}

fn profile_sort(c: &mut Criterion) {
    let mut profiles: Vec<_> = (0..500)
        .map(|index| profile(&format!("server-{index:03}")))
        .collect();
    c.bench_function("sort_500_profiles", |b| {
        b.iter(|| {
            profiles.reverse();
            sort_profiles(&mut profiles);
        });
    });
}

criterion_group!(benches, profile_sort);
criterion_main!(benches);
