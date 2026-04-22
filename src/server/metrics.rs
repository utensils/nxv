//! In-memory runtime metrics: rolling request-latency window, per-minute
//! activity ring buffer, process start time, and a total-request counter.
//!
//! All state is in-memory and lost on restart — the front end labels the
//! corresponding panels "activity · 30m" and "uptime since start", so the
//! numbers remain honest.

use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Number of most-recent request durations kept for percentile computation.
const LATENCY_WINDOW: usize = 1024;
/// Number of one-minute activity buckets reported (last 30 minutes).
pub const ACTIVITY_BUCKETS: usize = 30;

/// Aggregate metrics store. Shared across request handlers via `AppState`.
pub struct MetricsStore {
    started_at: SystemTime,
    total_requests: AtomicU64,
    /// Rolling window of per-request latencies in microseconds, for API routes only.
    latency_us: Mutex<VecDeque<u32>>,
    /// Sparse per-minute counts over all HTTP traffic, newest at the back.
    activity: Mutex<VecDeque<(u64, u64)>>,
}

impl MetricsStore {
    pub fn new() -> Self {
        Self {
            started_at: SystemTime::now(),
            total_requests: AtomicU64::new(0),
            latency_us: Mutex::new(VecDeque::with_capacity(LATENCY_WINDOW)),
            activity: Mutex::new(VecDeque::with_capacity(ACTIVITY_BUCKETS + 1)),
        }
    }

    /// Record a completed request.
    ///
    /// `is_api` gates inclusion in the latency percentile window so static-asset
    /// responses (which are sub-ms) don't distort the reported API latency.
    pub fn record(&self, elapsed_micros: u128, is_api: bool) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        let now_secs = now_epoch_secs();
        if let Ok(mut act) = self.activity.lock() {
            record_activity(&mut act, now_secs);
        }

        if is_api && let Ok(mut buf) = self.latency_us.lock() {
            let micros = elapsed_micros.min(u32::MAX as u128) as u32;
            if buf.len() >= LATENCY_WINDOW {
                buf.pop_front();
            }
            buf.push_back(micros);
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let now_secs = now_epoch_secs();
        let uptime = SystemTime::now()
            .duration_since(self.started_at)
            .unwrap_or_default();

        // Latency percentiles
        let (p50_ms, p95_ms, p99_ms, samples) = match self.latency_us.lock() {
            Ok(buf) => {
                let mut sorted: Vec<u32> = buf.iter().copied().collect();
                sorted.sort_unstable();
                let samples = sorted.len() as u64;
                let pct = |q: f64| -> f64 {
                    if sorted.is_empty() {
                        return 0.0;
                    }
                    let idx = ((sorted.len() as f64 - 1.0) * q).round() as usize;
                    sorted[idx.min(sorted.len() - 1)] as f64 / 1000.0
                };
                (pct(0.50), pct(0.95), pct(0.99), samples)
            }
            Err(_) => (0.0, 0.0, 0.0, 0),
        };

        // Activity: materialize a dense 30-entry array ending at the current minute
        let activity = match self.activity.lock() {
            Ok(buf) => snapshot_activity(&buf, now_secs),
            Err(_) => (0..ACTIVITY_BUCKETS as u64)
                .map(|i| ActivityBucket {
                    minute: minute_to_datetime(
                        now_secs - now_secs % 60 - (ACTIVITY_BUCKETS as u64 - 1 - i) * 60,
                    ),
                    count: 0,
                })
                .collect(),
        };

        Snapshot {
            started_at: DateTime::<Utc>::from(self.started_at),
            uptime_seconds: uptime.as_secs(),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            p50_ms,
            p95_ms,
            p99_ms,
            latency_samples: samples,
            activity,
        }
    }
}

impl Default for MetricsStore {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Snapshot {
    pub started_at: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub latency_samples: u64,
    pub activity: Vec<ActivityBucket>,
}

pub struct ActivityBucket {
    pub minute: DateTime<Utc>,
    pub count: u64,
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn minute_to_datetime(minute_secs: u64) -> DateTime<Utc> {
    DateTime::<Utc>::from(UNIX_EPOCH + std::time::Duration::from_secs(minute_secs))
}

fn record_activity(buf: &mut VecDeque<(u64, u64)>, now_secs: u64) {
    let minute = now_secs - now_secs % 60;
    let cutoff = minute.saturating_sub(60 * (ACTIVITY_BUCKETS as u64 - 1));
    while let Some(&(m, _)) = buf.front() {
        if m < cutoff {
            buf.pop_front();
        } else {
            break;
        }
    }
    if let Some(back) = buf.back_mut()
        && back.0 == minute
    {
        back.1 += 1;
        return;
    }
    buf.push_back((minute, 1));
}

fn snapshot_activity(buf: &VecDeque<(u64, u64)>, now_secs: u64) -> Vec<ActivityBucket> {
    let minute = now_secs - now_secs % 60;
    (0..ACTIVITY_BUCKETS as u64)
        .map(|i| {
            let m = minute - (ACTIVITY_BUCKETS as u64 - 1 - i) * 60;
            let count = buf
                .iter()
                .find(|(bm, _)| *bm == m)
                .map(|(_, c)| *c)
                .unwrap_or(0);
            ActivityBucket {
                minute: minute_to_datetime(m),
                count,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_activity_appends_and_evicts() {
        let mut buf = VecDeque::new();
        // t = 1000s → minute 960
        record_activity(&mut buf, 1000);
        record_activity(&mut buf, 1010);
        assert_eq!(buf.back().unwrap().1, 2);

        // skip ahead to minute 60*35 → anything older than 29 mins should fall off
        let new_now = 960 + 60 * 35;
        record_activity(&mut buf, new_now);
        assert!(buf.front().unwrap().0 >= new_now - new_now % 60 - 60 * 29);
    }

    #[test]
    fn snapshot_gives_dense_30_buckets_even_when_idle() {
        let buf = VecDeque::new();
        let out = snapshot_activity(&buf, 10_000);
        assert_eq!(out.len(), ACTIVITY_BUCKETS);
        assert!(out.iter().all(|b| b.count == 0));
    }

    #[test]
    fn percentile_math() {
        let store = MetricsStore::new();
        for us in [1000u128, 2000, 3000, 4000, 5000] {
            store.record(us, true);
        }
        let s = store.snapshot();
        // p50 of [1..5]ms should be 3ms
        assert!((s.p50_ms - 3.0).abs() < 0.001);
        assert_eq!(s.latency_samples, 5);
    }
}
