# Changelog

## Unreleased

### Added
- **GPU benches (AMD / Vulkan)**: real `ash` backend for buffer-copy bandwidth, compute throughput, and reduction/hash-style workloads; Kraft Index GPU weight when measurements exist; honest Unsupported when no AMD Vulkan device (`kraftverk-bench` feature `gpu`, default on)
- **OS-backed sensors**: Linux hwmon + RAPL; Windows ACPI thermal zones when present; telemetry/`--max-temp`/`--max-power` consume real readings only — never fabricated (`docs/telemetry.md`)
- **Privileged agent (operational)**: authenticated local IPC (named pipe / Unix socket), auth token handshake, apply/verify/rollback for `process.priority`, `process.affinity`, `power.scheme`; CLI `kraftverk agent serve|status`; doctor reports agent OK/FAIL
- **Search plugins**: ε-greedy multi-armed bandit and Bayesian GP+EI strategies selectable via `optimize --strategy`
- **Tauri desktop packaging**: optional `tauri-app` feature + `tauri.conf.json`; default remains CI-safe `web-server` axum UI

### Changed
- **License**: replaced MIT with proprietary **All Rights Reserved** terms (`LicenseRef-Proprietary`). Copyright holder: **theworker02**. View/use as published; no modification or redistribution of modified versions without written permission. See [LICENSE](LICENSE).
- README / site / docs: professional presence (Swedish etymology, architecture diagram, badges, FUNDING.yml, CODE_OF_CONDUCT).
- Cargo workspace `license` set to `LicenseRef-Proprietary`; authors attributed to `theworker02`.

> Patch release `0.2.2` recommended so artifacts advertise proprietary LICENSE plus these gap fixes.

## 0.2.1 — AMD-Exclusive Hardware Enforcement

### Added
- Central `HardwareEligibility` subsystem in `kraftverk-system` (CPUID + PCI vendor IDs)
- Policy `amd-only-v1` persisted on experiments and fingerprints
- CLI shared gate + inspect-only `compatibility` / `hardware` / `amd`
- Exit codes 20–25 for unsupported hardware
- Desktop blocking screen + `/api/eligibility`
- SDK `Kraftverk::open_default()` fails with `UnsupportedHardware`
- Privileged agent startup + sensitive-op revalidation
- Hot-plug NVIDIA abort (stop / restore / block)
- Mock combination eligibility tests
- Docs: `docs/hardware-support.md`, Pages `/docs/hardware-support`

### Changed
- README / branding / safety clarify AMD-exclusive specialization (no AMD impersonation)
- Windows GPU enumeration also matches display ClassGUID (in addition to Class=Display)

## 0.2.0 — Expansive Platform Phase

### Added
- Crate consolidation into ≤10 crates (`system`, `optimizer`, `data`, `sdk`, `desktop`)
- KraftBench v2: CPU scaling, compile proxy, responsiveness index, sustained windows (`--sustained`)
- Expanded telemetry with environmental noise model (no fabricated temps/power)
- Optimizer goals, constraints, sessions/resume, lineage, insights, objectives, search plugins, evidence receipts
- CLI expansions (`doctor`, `capabilities`, `report`, `chase`, `analyze`, `receipt`, `profile *`), HTML/JSON reports
- Desktop engineering instrument + Safety Center, branding, CI/Pages/Release, SDK facade

### Honest limitations
- GPU benchmarks unsupported (no vendor backend)
- Temperature/power sensors unavailable portably
- Privileged agent remains scaffold
- Desktop is local web UI (Tauri packaging later)
- ε-greedy / Bayesian search plugins listed but not implemented

## 0.1.0 — Milestone 1

Initial measurable foundation: KraftBench, Kraft Index, safe hill-climb optimize, SQLite history, restore.
