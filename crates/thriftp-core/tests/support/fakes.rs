//! Deterministic test fakes: clock, audit sink, host, and fault injection.

#![allow(dead_code)]

use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

/// Controllable clock for expiry, retention, and TTL tests.
#[derive(Debug)]
pub struct FakeClock {
    now: Mutex<DateTime<Utc>>,
}

impl FakeClock {
    #[must_use]
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    #[must_use]
    pub fn now(&self) -> DateTime<Utc> {
        *self.now.lock().expect("clock")
    }

    pub fn advance(&self, seconds: i64) {
        let mut now = self.now.lock().expect("clock");
        *now += Duration::seconds(seconds);
    }
}

/// In-memory audit sink that records redacted operation summaries.
#[derive(Default)]
pub struct InMemoryAuditSink {
    records: Mutex<Vec<Value>>,
    fail: Mutex<bool>,
}

impl InMemoryAuditSink {
    pub fn record(&self, operation: &str, result: &str) -> Result<(), String> {
        if *self.fail.lock().expect("audit") {
            return Err("audit_unavailable".to_owned());
        }
        self.records
            .lock()
            .expect("audit")
            .push(serde_json::json!({"operation": operation, "result": result}));
        Ok(())
    }

    pub fn fail_closed(&self, fail: bool) {
        *self.fail.lock().expect("audit") = fail;
    }

    #[must_use]
    pub fn records(&self) -> Vec<Value> {
        self.records.lock().expect("audit").clone()
    }
}

/// Fake host endpoint used by control-client tests.
#[derive(Debug, Clone)]
pub struct FakeHost {
    pub endpoint: String,
    pub token: String,
    pub pid: u32,
}

impl FakeHost {
    #[must_use]
    pub fn local() -> Self {
        Self {
            endpoint: "unix:/tmp/fake-host.sock".to_owned(),
            token: "fake-token".to_owned(),
            pid: std::process::id(),
        }
    }
}

/// Fault-injection switches shared by test fakes.
#[derive(Debug, Default, Clone)]
pub struct FaultInjection {
    pub audit_unavailable: bool,
    pub network_failure: bool,
    pub permission_denied: bool,
    pub revoked: bool,
}

impl FaultInjection {
    #[must_use]
    pub fn healthy() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn audit_down() -> Self {
        Self {
            audit_unavailable: true,
            ..Self::default()
        }
    }
}
