# Architecture (Milestone 1)

## Crate layout (consolidated)

Suggested fine-grained crates were merged where boundaries were artificial for M1:

| Crate | Role | Absorbed from |
|-------|------|----------------|
| `kraftverk-core` | Domain models, errors, stats, Kraft Index, goals/constraints | — |
| `kraftverk-system` | Platform trait, Native/Mock, recovery journal, hardware inspect, telemetry | platform + hardware + telemetry |
| `kraftverk-bench` | KraftBench workloads + suite runner | — |
| `kraftverk-optimizer` | Search strategies + profile catalog | search + profiles |
| `kraftverk-data` | SQLite experiment store + data paths | storage |
| `kraftverk-agent` | Privileged agent IPC types (scaffold) | — |
| `kraftverk-cli` | User-facing binary / optimize orchestration | — |

`_archive_m1_crates/` retains the pre-consolidation crate trees for reference only (not workspace members).

### Why consolidate

- Platform + hardware + telemetry always travel together for inspect/optimize sessions.
- Search + profiles are both “what to try next” concerns.
- Storage is the only persistence backend in M1; keeping it as `kraftverk-data` leaves room for reports/exports without resurrecting six tiny crates.

## Control flow

Evidence loop (product framing): **Measure → Experiment → Validate → Improve** (learn / repeat). See [assets/architecture-flow.svg](../assets/architecture-flow.svg).

```
CLI → open_session (inspect fingerprint, open DB, recover journal)
    → baseline/benchmark: KraftBench samples → Kraft Index → SQLite
    → optimize: SearchStrategy proposes Candidate
         → ApplyGuard (journal + apply/verify)
         → KraftBench samples → compare vs baseline
         → rollback (search) or commit (validated accept)
```

## Platform boundary

Callers never branch on OS. `Platform` (in `kraftverk-system`) exposes:

- `capabilities()` / `topology()`
- `read_param` / `apply_change` / `verify_change` / `rollback_change`
- `score_multiplier()` (MockPlatform only; native = 1.0)

`MockPlatform` simulates deltas, noise, failed applies, throttling, and unsupported keys for tests.

## Persistence

SQLite schema stores fingerprint, versions, OS info, candidate JSON, parent id, samples, stats, telemetry, stability, score, accept/reject reason, timestamps. See `kraftverk-data`.

## Privilege separation

M1 safe opts run in-process. `kraftverk-agent` defines request/response types for a future authenticated local IPC channel. See `docs/security.md`.
