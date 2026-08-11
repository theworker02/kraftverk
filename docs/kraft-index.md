# Kraft Index

## Definition

The Kraft Index is a **weighted composite** of category scores derived from KraftBench measurements.

1. Each measurement contributes a higher-is-better `score`
2. Within a category, scores are combined with a **geometric mean** (limits one microbench dominating)
3. Categories are blended with documented weights
4. Missing categories (e.g. GPU weight 0, or failed storage) cause **renormalization** over present weights
5. A baseline run’s mean raw composite maps to **10,000**
6. Later runs: `index = (raw / baseline_raw) * 10000`

## Default weights (M1)

| Category | Weight | Rationale |
|----------|--------|-----------|
| CPU | 0.40 | Dominant for general systems work |
| Memory | 0.20 | Bandwidth/latency sensitivity |
| Storage | 0.15 | Local I/O capability |
| System | 0.10 | Scheduler / threading proxies |
| Realtime | 0.15 | Pipeline / parse / parallel stand-ins |
| GPU | 0.00 | Reserved until a real backend exists |

Weights sum to 1.0. GPU is zero so we do not dilute the index with placeholders.

## Storage

Every experiment stores:

- Composite Kraft Index
- Category scores
- Per-benchmark components
- All raw sample measurement sets

See `kraftverk explain <id>`.
