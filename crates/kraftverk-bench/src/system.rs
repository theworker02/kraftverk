//! System / scheduler proxies: thread create, sync, wake latency.

use std::sync::{mpsc, Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use kraftverk_core::measurement::{BenchmarkId, Measurement, MetricDirection};

use crate::workload_cfg::WorkloadConfig;

pub fn run_all(cfg: &WorkloadConfig) -> Vec<Measurement> {
    vec![thread_create(cfg), barrier_sync(cfg), wake_latency(cfg)]
}

fn thread_create(cfg: &WorkloadConfig) -> Measurement {
    let n = 200usize.min(50 * cfg.worker_threads.max(1));
    let start = Instant::now();
    let mut handles = Vec::with_capacity(n);
    for i in 0..n {
        handles.push(thread::spawn(move || i.wrapping_mul(3)));
    }
    let mut acc = 0usize;
    for h in handles {
        acc = acc.wrapping_add(h.join().unwrap_or(0));
    }
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let rate = n as f64 / elapsed;
    Measurement {
        id: BenchmarkId::new("system.thread_create"),
        category: "system".into(),
        score: rate,
        raw_value: rate,
        unit: "threads/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(format!("{acc}")),
        notes: vec![],
    }
}

fn barrier_sync(cfg: &WorkloadConfig) -> Measurement {
    let workers = cfg.worker_threads.clamp(2, 32);
    let rounds = 200usize;
    let barrier = Arc::new(Barrier::new(workers));
    let start = Instant::now();
    let mut handles = Vec::new();
    for _ in 0..workers {
        let b = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            for _ in 0..rounds {
                b.wait();
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let ops = (workers * rounds) as f64 / elapsed;
    Measurement {
        id: BenchmarkId::new("system.barrier_sync"),
        category: "system".into(),
        score: ops,
        raw_value: ops,
        unit: "waits/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: None,
        notes: vec![format!("workers={workers}")],
    }
}

fn wake_latency(_cfg: &WorkloadConfig) -> Measurement {
    let iters = 500usize;
    let (tx, rx) = mpsc::sync_channel::<Instant>(0);
    let handle = thread::spawn(move || {
        let mut samples = Vec::with_capacity(iters);
        for _ in 0..iters {
            // Block until sender is ready, then record wake delay from token time.
            match rx.recv() {
                Ok(t0) => samples.push(t0.elapsed()),
                Err(_) => break,
            }
        }
        samples
    });
    thread::sleep(Duration::from_millis(5));
    for _ in 0..iters {
        let t0 = Instant::now();
        // sync_channel(0) rendezvous: blocks until receiver accepts.
        let _ = tx.send(t0);
    }
    drop(tx);
    let samples = handle.join().unwrap_or_default();
    let mean_ns = if samples.is_empty() {
        0.0
    } else {
        samples.iter().map(|d| d.as_nanos() as f64).sum::<f64>() / samples.len() as f64
    };
    // Lower latency is better → oriented score = 1/seconds.
    let mean_s = mean_ns / 1e9;
    Measurement {
        id: BenchmarkId::new("system.wake_latency"),
        category: "system".into(),
        score: Measurement::oriented_score(mean_s, MetricDirection::LowerIsBetter),
        raw_value: mean_ns,
        unit: "ns".into(),
        direction: MetricDirection::LowerIsBetter,
        checksum: None,
        notes: vec!["mpsc rendezvous wake proxy".into()],
    }
}
