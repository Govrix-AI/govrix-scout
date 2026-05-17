//! Probabilistic sketches per (agent, model) pair.
//!
//! Each `SketchBundle` is a bounded collection of online estimators used by the
//! post-flush detectors:
//!
//! - **EWMA / EWMSD** — exponential mean & standard deviation of cost (z-score).
//! - **t-digest** — streaming quantile estimator for cost + latency (p50/p95/p99).
//! - **MiniHll** — tiny inline HyperLogLog-flavoured cardinality estimator
//!   for tools/providers/error categories. The original `hyperloglog-rs 0.1`
//!   crate listed in workspace deps requires nightly features (`const_float_bits_conv`)
//!   and refuses to build on stable, so we ship a small dependency-free
//!   approximation here.
//! - **Off-hours histogram** — 24-bucket count of activity by UTC hour.
//! - **Error / token windows** — fixed-size rings for recent rates.
//!
//! All updates are O(1) (amortised). Detectors read the sketch *before* the
//! update so a single anomaly cannot mask itself.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use chrono::{DateTime, Timelike, Utc};
use tdigest::TDigest;

/// EWMA alpha used for cost mean/variance. Smaller = slower adaptation.
const EWMA_ALPHA: f64 = 0.1;

/// Rolling window size for token / error rate detectors.
const ROLLING_WINDOW: usize = 100;

/// Per-(agent, model) sketch bundle. `~3 KB` per entry in steady state.
#[derive(Debug, Clone)]
pub struct SketchBundle {
    /// Exponentially weighted moving mean of cost (USD per event).
    pub cost_mean: f64,
    /// Exponentially weighted moving variance of cost.
    pub cost_var: f64,
    /// Long-window mean of cost (slower α) for `RapidCostEscalation`.
    pub cost_mean_slow: f64,
    /// Total events observed (after warm-up the detectors trust the sketch).
    pub samples: u64,

    /// Streaming quantile estimator over per-event latency_ms.
    pub latency_digest: TDigest,
    /// Streaming quantile estimator over per-event cost_usd.
    pub cost_digest: TDigest,

    /// Distinct tool/provider/etc cardinality sketch.
    pub tool_cardinality: MiniHll,
    /// EWMA mean of "unique tools seen in last 100 events" (for fan-out alerts).
    pub fanout_mean: f64,
    /// EWMA variance of the same.
    pub fanout_var: f64,

    /// 24-bucket count of activity by UTC hour.
    pub hour_histogram: [u64; 24],

    /// EMA of total tokens per event (carry-over from legacy TokenVolume).
    pub token_ema: f64,

    /// Fixed-size ring of recent error flags (1 = error, 0 = ok).
    pub error_window: VecRing,

    /// Distinct provider names seen for this agent (cardinality < 10, plain set).
    pub providers_seen: HashSet<String>,

    /// Last time this bundle was touched, for LRU eviction debugging.
    pub last_seen: DateTime<Utc>,
}

impl SketchBundle {
    /// Empty bundle — used for first event seen.
    pub fn new() -> Self {
        Self {
            cost_mean: 0.0,
            cost_var: 0.0,
            cost_mean_slow: 0.0,
            samples: 0,
            latency_digest: TDigest::new_with_size(100),
            cost_digest: TDigest::new_with_size(100),
            tool_cardinality: MiniHll::new(),
            fanout_mean: 0.0,
            fanout_var: 0.0,
            hour_histogram: [0; 24],
            token_ema: 0.0,
            error_window: VecRing::new(ROLLING_WINDOW),
            providers_seen: HashSet::new(),
            last_seen: Utc::now(),
        }
    }

    /// Seed a bundle from aggregate stats pulled from the events table.
    ///
    /// `stddev` may be `None` (Postgres returns NULL for groups with <2 rows);
    /// callers should treat that as warm-up and let the detector ramp on live
    /// traffic.
    pub fn from_seed(
        avg_cost: f64,
        stddev_cost: Option<f64>,
        avg_latency: f64,
        stddev_latency: Option<f64>,
        samples: u64,
    ) -> Self {
        let mut b = Self::new();
        b.cost_mean = avg_cost;
        b.cost_mean_slow = avg_cost;
        b.cost_var = stddev_cost.map(|s| s * s).unwrap_or(0.0);
        b.samples = samples;
        // Seed the latency digest with two synthetic points so a quantile query
        // returns the seeded mean ± 1σ; live traffic overwrites this quickly.
        let lo = (avg_latency - stddev_latency.unwrap_or(0.0)).max(0.0);
        let hi = avg_latency + stddev_latency.unwrap_or(0.0);
        if samples > 0 {
            b.latency_digest = b.latency_digest.merge_unsorted(vec![lo, avg_latency, hi]);
            b.cost_digest = b.cost_digest.merge_unsorted(vec![avg_cost]);
        }
        b
    }

    /// Update the EWMA mean/variance using Welford-style online update.
    pub fn update_cost(&mut self, cost: f64) {
        let alpha = EWMA_ALPHA;
        let delta = cost - self.cost_mean;
        self.cost_mean += alpha * delta;
        // EWMSD: var ← (1-α)·(var + α·delta²)
        self.cost_var = (1.0 - alpha) * (self.cost_var + alpha * delta * delta);
        // Slow channel: 5× slower α to detect sustained drift.
        let alpha_slow = alpha * 0.2;
        self.cost_mean_slow += alpha_slow * (cost - self.cost_mean_slow);
    }

    /// Update t-digest streams with a single observation.
    pub fn update_digests(&mut self, latency_ms: Option<u32>, cost: f64) {
        if let Some(lm) = latency_ms {
            self.latency_digest = self.latency_digest.merge_unsorted(vec![lm as f64]);
        }
        if cost > 0.0 {
            self.cost_digest = self.cost_digest.merge_unsorted(vec![cost]);
        }
    }

    /// Update the fanout EWMA after a tool is seen.
    pub fn update_fanout(&mut self, unique_tools_estimate: f64) {
        let alpha = EWMA_ALPHA;
        let delta = unique_tools_estimate - self.fanout_mean;
        self.fanout_mean += alpha * delta;
        self.fanout_var = (1.0 - alpha) * (self.fanout_var + alpha * delta * delta);
    }

    /// Standard deviation of cost.
    pub fn cost_stddev(&self) -> f64 {
        self.cost_var.sqrt()
    }

    /// Rolling error rate in [0, 1].
    pub fn error_rate(&self) -> f64 {
        self.error_window.mean()
    }

    /// p99 latency estimate. Returns 0.0 if no observations yet.
    pub fn p99_latency(&self) -> f64 {
        self.latency_digest.estimate_quantile(0.99)
    }
}

impl Default for SketchBundle {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tiny inline HLL ──────────────────────────────────────────────────────────

/// Minimal HyperLogLog approximation.
///
/// Uses 64 registers (precision = 6) tracking the position of the leading 1-bit
/// of `hash(item) >> 6`. Standard HLL estimator with bias correction:
/// `α_m · m² / Σ 2^(-Rj)`. Relative error ≈ 1.04 / √m ≈ 13 % at m=64 — fine for
/// "is the agent talking to 5× as many tools as usual" detection.
#[derive(Debug, Clone)]
pub struct MiniHll {
    registers: [u8; Self::M],
}

impl MiniHll {
    const M: usize = 64; // 2^6
    const PRECISION: u32 = 6;
    const ALPHA: f64 = 0.709; // α_64

    pub fn new() -> Self {
        Self {
            registers: [0; Self::M],
        }
    }

    /// Insert an item.
    pub fn insert<H: Hash>(&mut self, item: &H) {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        let h = hasher.finish();

        let idx = (h & ((Self::M as u64) - 1)) as usize;
        let w = h >> Self::PRECISION;
        // Position of leading 1 bit (1-based). leading_zeros on 58-bit w.
        let rank = if w == 0 {
            (64 - Self::PRECISION) as u8 + 1
        } else {
            (w.leading_zeros() - Self::PRECISION) as u8 + 1
        };
        if rank > self.registers[idx] {
            self.registers[idx] = rank;
        }
    }

    /// Estimate cardinality of the inserted set.
    pub fn estimate(&self) -> f64 {
        let m = Self::M as f64;
        let sum: f64 = self
            .registers
            .iter()
            .map(|&r| 2.0_f64.powi(-(r as i32)))
            .sum();
        let raw = Self::ALPHA * m * m / sum;
        // Small-range correction: linear counting when many zero registers.
        let zeros = self.registers.iter().filter(|&&r| r == 0).count() as f64;
        if raw <= 2.5 * m && zeros > 0.0 {
            m * (m / zeros).ln()
        } else {
            raw
        }
    }
}

impl Default for MiniHll {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tiny ring buffer ─────────────────────────────────────────────────────────

/// Fixed-capacity ring buffer of f64 used for rolling rate calcs.
#[derive(Debug, Clone)]
pub struct VecRing {
    buf: Vec<f64>,
    cap: usize,
    head: usize,
    len: usize,
}

impl VecRing {
    pub fn new(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap),
            cap,
            head: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, v: f64) {
        if self.buf.len() < self.cap {
            self.buf.push(v);
            self.len += 1;
        } else {
            self.buf[self.head] = v;
            self.head = (self.head + 1) % self.cap;
        }
    }

    pub fn mean(&self) -> f64 {
        if self.len == 0 {
            0.0
        } else {
            self.buf.iter().sum::<f64>() / self.len as f64
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Increment the hour-histogram bucket for the event's UTC hour.
pub fn record_hour(bundle: &mut SketchBundle, ts: DateTime<Utc>) {
    let h = ts.hour() as usize;
    bundle.hour_histogram[h] = bundle.hour_histogram[h].saturating_add(1);
    bundle.last_seen = ts;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minihll_estimates_small_cardinality() {
        let mut hll = MiniHll::new();
        for i in 0..20_u64 {
            hll.insert(&i);
        }
        let est = hll.estimate();
        assert!(est > 8.0 && est < 60.0, "estimate way off: {est}");
    }

    #[test]
    fn minihll_handles_duplicates() {
        let mut hll = MiniHll::new();
        for _ in 0..100 {
            hll.insert(&"same-key");
        }
        let est = hll.estimate();
        assert!(est < 5.0, "duplicates should yield ~1: got {est}");
    }

    #[test]
    fn vecring_mean_after_overflow() {
        let mut r = VecRing::new(3);
        r.push(1.0);
        r.push(2.0);
        r.push(3.0);
        r.push(4.0); // evicts the 1.0
        assert!((r.mean() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn ewma_converges() {
        let mut b = SketchBundle::new();
        for _ in 0..200 {
            b.update_cost(1.0);
        }
        assert!((b.cost_mean - 1.0).abs() < 0.01);
        assert!(b.cost_stddev() < 0.01);
    }

    #[test]
    fn from_seed_sets_mean() {
        let b = SketchBundle::from_seed(2.5, Some(0.5), 200.0, Some(50.0), 100);
        assert!((b.cost_mean - 2.5).abs() < 1e-9);
        assert!(b.samples == 100);
        assert!(b.cost_stddev() > 0.0);
    }
}
