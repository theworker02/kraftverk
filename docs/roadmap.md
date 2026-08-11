# Roadmap

## Milestone 1 — Complete

- Cargo workspace + docs
- Inspect, KraftBench, stats, Kraft Index
- SQLite experiments
- Safe optimize + rollback + recovery
- Core CLI surface

## Expansive Platform Phase (0.2) — Complete

- ≤10 crates (`system`, `optimizer`, `data`, `sdk`, `desktop` consolidation)
- KraftBench v2, telemetry/noise model, optimizer goals/sessions/profiles/receipts
- HTML/JSON reports, doctor/capabilities/insights, desktop instrument UI
- Branding, Pages site, CI + release workflows

## Milestone 2 (planned)

- Privileged agent with authenticated IPC
- Power-plan / selected system tunables where reversible
- Richer telemetry sensors (where APIs exist)
- Additional search strategies (ε-greedy, Bayesian opt — still seedable)
- Optional Tauri packaging for the desktop instrument

## Milestone 3+

- GPU backends (CUDA/Vulkan/Metal) with real kernels — never fake
- Cross-run regression dashboards (still local-first)
- Optional export formats for papers / CI

## Explicit non-roadmap

- Placebo “cleaner” features
- Undocumented silent tweaks
- Cheating benchmarks to inflate marketing numbers
