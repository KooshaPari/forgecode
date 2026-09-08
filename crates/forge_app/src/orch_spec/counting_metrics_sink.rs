//! Test-only [`MetricsSink`] that captures counter increments keyed by metric
//! name so e2e tests can assert that the orchestrator hit a given telemetry
//! code path exactly the expected number of times.
//!
//! Used by the bounded-window truncation spec to lock in that
//! `forge.length_truncation` fires once per `MaxTokensReached` interrupt — a
//! regression of the primary path (the orchestrator's error downcast) silently
//! skipping the counter would otherwise be invisible.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use forge_domain::{MetricsSink, metric_names};

/// Shared, in-memory counter store.
#[derive(Default, Clone)]
pub struct CountingMetricsSink {
    counters: Arc<Mutex<HashMap<&'static str, u64>>>,
}

impl CountingMetricsSink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot the current counter value for `name`.
    pub fn counter(&self, name: &'static str) -> u64 {
        *self
            .counters
            .lock()
            .expect("counting sink mutex poisoned")
            .get(&name)
            .unwrap_or(&0)
    }

    /// Convenience accessor for the bounded-window metric.
    pub fn length_truncations(&self) -> u64 {
        self.counter(metric_names::LENGTH_TRUNCATION)
    }
}

impl MetricsSink for CountingMetricsSink {
    fn increment(&self, name: &'static str, delta: u64) {
        if delta == 0 {
            return;
        }
        let mut guard = self.counters.lock().expect("counting sink mutex poisoned");
        *guard.entry(name).or_insert(0) += delta;
    }

    fn record_duration(&self, _name: &'static str, _duration: std::time::Duration) {
        // durations are not asserted by any current spec
    }

    fn record_error(&self, _name: &'static str) {
        // not asserted by any current spec
    }
}
