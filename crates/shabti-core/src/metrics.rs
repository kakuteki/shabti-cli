use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

/// Thread-safe engine metrics collector.
pub struct EngineMetrics {
    store_count: AtomicU64,
    search_count: AtomicU64,
    error_count: AtomicU64,
    search_latency_sum_ms: AtomicU64,
}

impl EngineMetrics {
    pub fn new() -> Self {
        Self {
            store_count: AtomicU64::new(0),
            search_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            search_latency_sum_ms: AtomicU64::new(0),
        }
    }

    pub fn record_store(&self) {
        self.store_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_search(&self, latency_ms: u64) {
        self.search_count.fetch_add(1, Ordering::Relaxed);
        self.search_latency_sum_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn store_count(&self) -> u64 {
        self.store_count.load(Ordering::Relaxed)
    }

    pub fn search_count(&self) -> u64 {
        self.search_count.load(Ordering::Relaxed)
    }

    pub fn error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }

    pub fn total_requests(&self) -> u64 {
        self.store_count() + self.search_count()
    }

    pub fn avg_search_latency_ms(&self) -> u64 {
        let count = self.search_count();
        if count == 0 {
            return 0;
        }
        self.search_latency_sum_ms.load(Ordering::Relaxed) / count
    }

    pub fn reset(&self) {
        self.store_count.store(0, Ordering::Relaxed);
        self.search_count.store(0, Ordering::Relaxed);
        self.error_count.store(0, Ordering::Relaxed);
        self.search_latency_sum_ms.store(0, Ordering::Relaxed);
    }

    pub fn to_json(&self) -> Value {
        json!({
            "store_count": self.store_count(),
            "search_count": self.search_count(),
            "error_count": self.error_count(),
            "total_requests": self.total_requests(),
            "avg_search_latency_ms": self.avg_search_latency_ms(),
        })
    }
}

impl Default for EngineMetrics {
    fn default() -> Self {
        Self::new()
    }
}
