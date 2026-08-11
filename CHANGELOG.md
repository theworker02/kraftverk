# Changelog

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

## 0.2.0 — Expansive Platform Phase

### Added
- Crate consolidation into ≤10 crates (`system`, `optimizer`, `data`, `sdk`, `desktop`)
- KraftBench v2: CPU scaling, compile proxy, responsiveness index, sustained windows (`--sustained`)
- Expanded telemetry with environmental noise model (no fabricated temps/power)
- Optimizer goals, constraints, sessions/resume, lineage, insights, objectives
- CLI expansions, HTML/JSON reports, desktop UI, branding, CI/Pages/Release, SDK

### Honest limitations
- GPU benchmarks unsupported (no vendor backend)
- Temperature/power sensors unavailable portably
- Privileged agent remains scaffold

## 0.1.0 — Milestone 1

Initial measurable foundation: KraftBench, Kraft Index, safe hill-climb optimize, SQLite history, restore.
