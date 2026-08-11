# Benchmarking (KraftBench v0)

KraftBench runs **real** deterministic workloads. Scores come from wall-clock timing of completed work with checksums where practical.

## Categories

| Category | Benches | Notes |
|----------|---------|-------|
| CPU | integer single/multi, float, hashing, compression | Multi uses configured worker/rayon pool |
| Memory | sequential R/W proxy, random chase, allocation | Proxies — not JEDEC bandwidth certs |
| Storage | seq/rand read/write | **Only** under Kraftverk scratch dir |
| System | thread create, barrier sync, wake latency | Scheduler proxies |
| Realtime | file pipeline, parallel reduce, parse | In-memory “real world” stand-ins |
| GPU | — | **Unsupported** in M1 (architecture reserved) |

## Methodology

1. Optional warm-up iterations (discarded)
2. N timed suite samples
3. Per-measurement higher-is-better orientation
4. Kraft Index from category geometric means + documented weights
5. Baseline normalizes mean raw composite → 10,000

## Storage safety

- Scratch path: platform data dir `/bench_scratch` with marker file `.kraftverk_bench_scratch`
- Refuses obvious user content roots (Documents/Desktop/Downloads)

## Reproducibility

- Seeded PRNGs (`ChaCha8`) for data generation
- Workload sizes fixed in code for M1
- Parallelism controlled via `bench.worker_threads` / `bench.rayon_threads`

## What we do not claim

- Absolute FLOPS / STREAM bandwidth equivalence
- Cross-machine ranking without identical suite versions
- GPU performance (no backend)
