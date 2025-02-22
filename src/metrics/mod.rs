use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct StreamingMetrics {
    pub capture: TimingMetrics,
    pub encode: TimingMetrics, 
    pub network: NetworkMetrics,
    pub decode: TimingMetrics,
    pub display: TimingMetrics,
}

#[derive(Default)]
pub struct TimingMetrics {
    pub last_duration: Arc<AtomicU64>,
    pub avg_duration: Arc<AtomicU64>,
    pub min_duration: Arc<AtomicU64>,
    pub max_duration: Arc<AtomicU64>,
    pub fps: Arc<AtomicU64>,
    pub total_frames: Arc<AtomicU64>,
    pub dropped_frames: Arc<AtomicU64>,
}

#[derive(Default)]
pub struct NetworkMetrics {
    pub latency: Arc<AtomicU64>,
    pub bandwidth: Arc<AtomicU64>,
    pub packet_loss: Arc<AtomicU64>,
    pub jitter: Arc<AtomicU64>,
}

impl StreamingMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    // Generic timing function that returns both duration and result
    pub fn time_operation<T, E, F>(&self, metrics: &TimingMetrics, op: F) -> (Duration, Result<T, E>)
    where
        F: FnOnce() -> Result<T, E>,
    {
        let start = Instant::now();
        let result = op();
        let duration = start.elapsed();
        
        // Update metrics
        metrics.last_duration.store(duration.as_micros() as u64, Ordering::Relaxed);
        metrics.total_frames.fetch_add(1, Ordering::Relaxed);
        
        if result.is_err() {
            metrics.dropped_frames.fetch_add(1, Ordering::Relaxed);
        }

        // Update min/max/avg
        let current_min = metrics.min_duration.load(Ordering::Relaxed);
        if current_min == 0 || (duration.as_micros() as u64) < current_min {
            metrics.min_duration.store(duration.as_micros() as u64, Ordering::Relaxed);
        }

        let current_max = metrics.max_duration.load(Ordering::Relaxed);
        if duration.as_micros() as u64 > current_max {
            metrics.max_duration.store(duration.as_micros() as u64, Ordering::Relaxed);
        }

        // Update rolling average
        let total = metrics.total_frames.load(Ordering::Relaxed);
        let avg = metrics.avg_duration.load(Ordering::Relaxed);
        let new_avg = if total > 1 {
            ((avg as f64 * (total-1) as f64) + duration.as_micros() as f64) / total as f64
        } else {
            duration.as_micros() as f64
        };
        metrics.avg_duration.store(new_avg as u64, Ordering::Relaxed);

        // Update FPS
        if duration.as_micros() > 0 {
            let fps = 1_000_000.0 / duration.as_micros() as f64;
            metrics.fps.store(fps as u64, Ordering::Relaxed);
        }

        (duration, result)
    }
}