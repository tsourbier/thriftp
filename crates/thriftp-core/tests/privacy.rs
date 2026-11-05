use std::path::Path;

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn no_network_or_telemetry_dependencies_are_enabled_by_default() {
    let manifest =
        std::fs::read_to_string(workspace_root().join("crates/thriftp-core/Cargo.toml")).unwrap();
    let protocols =
        std::fs::read_to_string(workspace_root().join("crates/thriftp-protocols/Cargo.toml"))
            .unwrap();
    for forbidden in [
        "reqwest",
        "ureq",
        "hyper",
        "sentry",
        "segment",
        "mixpanel",
        "amplitude",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "core must not depend on {forbidden}"
        );
        assert!(
            !protocols.contains(forbidden),
            "protocols must not depend on {forbidden}"
        );
    }
}

#[test]
fn preferences_do_not_request_accounts_or_telemetry() {
    let preference = thriftp_core::model::Preference::default();
    assert!(preference.restore_tabs);
    assert_eq!(preference.theme, thriftp_core::model::Theme::System);
    assert_eq!(preference.default_concurrency, 4);
    assert!(preference.bandwidth_limit_bytes_per_second.is_none());
    // Preferences contain no account, subscription, or telemetry fields.
    let serialized = serde_json::to_string(&preference).unwrap();
    for forbidden in ["account", "subscription", "telemetry", "analytics", "cloud"] {
        assert!(!serialized.to_lowercase().contains(forbidden));
    }
}

#[test]
fn source_has_no_telemetry_endpoint_literals() {
    let root = workspace_root();
    for relative in [
        "crates/thriftp-core/src/lib.rs",
        "crates/thriftp-core/src/connections.rs",
        "crates/thriftp-protocols/src/lib.rs",
    ] {
        let source = std::fs::read_to_string(root.join(relative)).unwrap();
        for forbidden in ["sentry.io", "analytics", "telemetry", "https://api."] {
            assert!(
                !source.to_lowercase().contains(forbidden),
                "{relative} contains {forbidden}"
            );
        }
    }
}
