use serde::{Deserialize, Serialize};

pub struct MetricsCollector {
    request_count: std::sync::atomic::AtomicU64,
    error_count: std::sync::atomic::AtomicU64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            request_count: std::sync::atomic::AtomicU64::new(0),
            error_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn record_request(&self) {
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.error_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> MetricsStats {
        MetricsStats {
            requests: self
                .request_count
                .load(std::sync::atomic::Ordering::Relaxed),
            error: self.error_count.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct MetricsStats {
    requests: u64,
    error: u64,
}
