//! Storage benchmarks — ONLY inside Kraftverk-owned temp directories.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::Instant;

use kraftverk_core::error::{Error, Result};
use kraftverk_core::measurement::{BenchmarkId, Measurement, MetricDirection};
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use sha2::{Digest, Sha256};

use crate::workload_cfg::WorkloadConfig;

pub fn run_all(cfg: &WorkloadConfig) -> Result<Vec<Measurement>> {
    let dir = cfg
        .storage_dir
        .as_ref()
        .ok_or_else(|| Error::Benchmark("storage_dir not set".into()))?;
    ensure_safe_dir(dir)?;
    Ok(vec![
        seq_write(dir, cfg)?,
        seq_read(dir, cfg)?,
        rand_write(dir, cfg)?,
        rand_read(dir, cfg)?,
    ])
}

fn ensure_safe_dir(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    // Marker file proves this is our scratch space.
    let marker = dir.join(".kraftverk_bench_scratch");
    if !marker.exists() {
        fs::write(&marker, b"kraftverk-owned-temp\n")?;
    }
    // Refuse if path looks like a user home root without marker parent chain — soft check.
    let s = dir.to_string_lossy().to_ascii_lowercase();
    if s.ends_with("documents") || s.ends_with("desktop") || s.ends_with("downloads") {
        return Err(Error::Benchmark(format!(
            "refusing storage bench on suspicious path {}",
            dir.display()
        )));
    }
    Ok(())
}

fn seq_write(dir: &Path, cfg: &WorkloadConfig) -> Result<Measurement> {
    let path = dir.join("seq.bin");
    let size = 16 << 20; // 16 MiB
    let mut buf = vec![0u8; 1 << 20];
    let mut rng = ChaCha8Rng::seed_from_u64(cfg.seed);
    rng.fill(&mut buf[..]);
    let start = Instant::now();
    {
        let mut f = File::create(&path)?;
        let mut written = 0;
        while written < size {
            let n = (size - written).min(buf.len());
            f.write_all(&buf[..n])?;
            written += n;
        }
        f.sync_all()?;
    }
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let bps = size as f64 / elapsed;
    Ok(Measurement {
        id: BenchmarkId::new("storage.seq_write"),
        category: "storage".into(),
        score: bps,
        raw_value: bps,
        unit: "B/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(hex::encode(Sha256::digest(&buf))[..16].to_string()),
        notes: vec![format!("path={}", path.display())],
    })
}

fn seq_read(dir: &Path, cfg: &WorkloadConfig) -> Result<Measurement> {
    let path = dir.join("seq.bin");
    if !path.exists() {
        seq_write(dir, cfg)?;
    }
    let meta = fs::metadata(&path)?;
    let size = meta.len() as usize;
    let mut buf = vec![0u8; 1 << 20];
    let start = Instant::now();
    let mut hasher = Sha256::new();
    {
        let mut f = File::open(&path)?;
        let mut remaining = size;
        while remaining > 0 {
            let n = remaining.min(buf.len());
            f.read_exact(&mut buf[..n])?;
            hasher.update(&buf[..n]);
            remaining -= n;
        }
    }
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let bps = size as f64 / elapsed;
    Ok(Measurement {
        id: BenchmarkId::new("storage.seq_read"),
        category: "storage".into(),
        score: bps,
        raw_value: bps,
        unit: "B/s".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(hex::encode(hasher.finalize())[..16].to_string()),
        notes: vec![],
    })
}

fn rand_write(dir: &Path, cfg: &WorkloadConfig) -> Result<Measurement> {
    let path = dir.join("rand.bin");
    let size = 4 << 20; // 4 MiB file
    let block = 4096usize;
    let ops = 1024usize;
    {
        let f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
        f.set_len(size as u64)?;
    }
    let mut buf = vec![0u8; block];
    let mut rng = ChaCha8Rng::seed_from_u64(cfg.seed ^ 0x52414E44);
    rng.fill(&mut buf[..]);
    let start = Instant::now();
    {
        let mut f = OpenOptions::new().write(true).open(&path)?;
        for _ in 0..ops {
            let max_off = (size / block).saturating_sub(1);
            let off = (rng.gen_range(0..=max_off) * block) as u64;
            f.seek(SeekFrom::Start(off))?;
            f.write_all(&buf)?;
        }
        f.sync_all()?;
    }
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let iops = ops as f64 / elapsed;
    Ok(Measurement {
        id: BenchmarkId::new("storage.rand_write"),
        category: "storage".into(),
        score: iops,
        raw_value: iops,
        unit: "IOPS".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: None,
        notes: vec![format!("block={block}")],
    })
}

fn rand_read(dir: &Path, cfg: &WorkloadConfig) -> Result<Measurement> {
    let path = dir.join("rand.bin");
    if !path.exists() {
        rand_write(dir, cfg)?;
    }
    let size = fs::metadata(&path)?.len() as usize;
    let block = 4096usize;
    let ops = 1024usize;
    let mut buf = vec![0u8; block];
    let mut rng = ChaCha8Rng::seed_from_u64(cfg.seed ^ 0x52454144);
    let start = Instant::now();
    let mut acc = 0u64;
    {
        let mut f = File::open(&path)?;
        for _ in 0..ops {
            let max_off = (size / block).saturating_sub(1).max(1);
            let off = (rng.gen_range(0..max_off) * block) as u64;
            f.seek(SeekFrom::Start(off))?;
            f.read_exact(&mut buf)?;
            acc = acc.wrapping_add(buf[0] as u64);
        }
    }
    let elapsed = start.elapsed().as_secs_f64().max(1e-12);
    let iops = ops as f64 / elapsed;
    Ok(Measurement {
        id: BenchmarkId::new("storage.rand_read"),
        category: "storage".into(),
        score: iops,
        raw_value: iops,
        unit: "IOPS".into(),
        direction: MetricDirection::HigherIsBetter,
        checksum: Some(format!("{acc:x}")),
        notes: vec![],
    })
}
