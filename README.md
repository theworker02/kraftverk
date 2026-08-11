# Kraftverk

**Evidence-driven systems performance platform** — measure real work, experiment with reversible settings, keep only what statistically improves, learn, repeat.

AMD-exclusive (`amd-only-v1`). Not a PC cleaner. Not a placebo FPS booster. Not a cloud service. **Independent product — not affiliated with or endorsed by AMD.**

## Philosophy

**Measure → Experiment → Benchmark → Validate → Compare → Keep or Revert → Learn → Repeat**

Every optimization must improve measured performance, efficiency, stability, latency, thermal behavior, or instrumentation for better decisions. No fabricated telemetry. No magic buttons.

## Quick start

```bash
cargo build --release -p kraftverk-cli
cargo run -p kraftverk-cli -- compatibility
cargo run -p kraftverk-cli -- hardware
cargo run -p kraftverk-cli -- inspect
cargo run -p kraftverk-cli -- baseline
cargo run -p kraftverk-cli -- optimize --mode safe --goal balanced
cargo run -p kraftverk-cli -- status
cargo run -p kraftverk-cli -- report --format html
cargo run -p kraftverk-cli -- restore --baseline
```

Desktop (same local DB):

```bash
cargo run -p kraftverk-desktop
```

## Hardware gate

Supported = **x86|x86_64** + **AMD CPU** + GPU config allowed (no GPU OK; any NVIDIA/Intel GPU fails). Exit codes **20–25**. Inspect-only without gate: `kraftverk compatibility`, `kraftverk hardware`, `kraftverk amd cpu|gpu`.

Full policy: [docs/hardware-support.md](docs/hardware-support.md).

## What works (0.2)

| Area | Status |
|------|--------|
| AMD eligibility gate (CLI/desktop/SDK/agent/optimizer) | Working |
| `compatibility` / `hardware` / `amd cpu|gpu` | Working (inspect-only) |
| `inspect` / `doctor` / `capabilities` | Working |
| KraftBench v2 (CPU scaling, compile proxy, responsiveness, sustained) | Working |
| `baseline` / `benchmark [--sustained 10m]` | Working |
| `optimize` with goals, constraints, sessions/resume + hot-plug recheck | Working |
| `history` / `explain` / `compare` / `lineage` / `insights` | Working |
| `profile` export/inspect/apply/validate/recommend | Working |
| `report` html/json | Working |
| `chase` / `analyze` | Working |
| Desktop instrument UI + hardware blocker | Working |
| Privileged agent / GPU benches | Scaffold / unsupported (honest) |

## Goals

`balanced` · `gaming` · `compile` · `workstation` · `throughput` · `latency` · `efficiency` · `sustained` · `quiet`

## Crate layout (9 first-party)

```
crates/
├── kraftverk-core/
├── kraftverk-system/      # platform + hardware eligibility + telemetry
├── kraftverk-bench/
├── kraftverk-optimizer/   # search + profiles + objectives
├── kraftverk-data/        # SQLite + reports + sessions
├── kraftverk-agent/
├── kraftverk-sdk/
├── kraftverk-cli/
└── kraftverk-desktop/
```

## Docs

- [docs/hardware-support.md](docs/hardware-support.md)
- [docs/platforms.md](docs/platforms.md)
- [DEVELOPMENT.md](DEVELOPMENT.md)
- [CONTRIBUTING.md](CONTRIBUTING.md)
- [docs/api-stability.md](docs/api-stability.md)
- [docs/branding.md](docs/branding.md)
- [docs/roadmap.md](docs/roadmap.md)
- [CHANGELOG.md](CHANGELOG.md)

## License

MIT — see [LICENSE](LICENSE).
