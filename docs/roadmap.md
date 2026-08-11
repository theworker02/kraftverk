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

## Milestone 2 — Largely complete (0.2.2+)

- Privileged agent with authenticated local IPC (named pipe / Unix socket)
- Power-plan / selected system tunables where reversible (`power.scheme`, priority, affinity)
- OS-backed telemetry sensors where APIs exist (hwmon/RAPL; Windows thermal zones)
- ε-greedy and Bayesian search strategies (seedable)
- Tauri packaging for the desktop instrument (optional `tauri-app` feature; default web UI remains)

## Remaining honest limits

- Windows package power / AMD GPU temp without ADL remain unset (not fabricated)
- GPU benches require AMD Vulkan; otherwise skipped with Unsupported
- Tauri native shell needs platform WebView deps — CI keeps default `web-server`
- macOS: inspect/bench only; no parity claim for agent/sensors/GPU

## Milestone 3+

- Richer GPU suites / optional vendor SDKs where license-clean
- Cross-run regression dashboards (still local-first)
- Optional export formats for papers / CI

## Explicit non-roadmap

- Placebo “cleaner” features
- Undocumented silent tweaks
- Cheating benchmarks to inflate marketing numbers
