# Changelog

## Unreleased

## 0.2.2 — Proprietary license + gap fixes (2026-08-12)

Advertises proprietary LICENSE on release artifacts and closes remaining Milestone 2 gaps. Continues the **AMD-exclusive** `amd-only-v1` gate from 0.2.1 (unchanged policy; not reintroduced here).

### Added
- **GPU benches (AMD / Vulkan)**: real `ash` backend for device-local buffer-copy bandwidth, compute throughput, and XOR-reduction kernels; discrete AMD preferred (PCI 0x1002); Kraft Index GPU weight when measurements exist; honest Unsupported when no AMD Vulkan device (`kraftverk-bench` feature `gpu`, default on)
- **OS-backed sensors**: Linux hwmon + RAPL; Windows ACPI thermal zones when present; telemetry/`--max-temp`/`--max-power` consume real readings only — never fabricated (`docs/telemetry.md`)
- **Privileged agent (operational)**: authenticated local IPC (named pipe / Unix socket), auth token handshake, apply/verify/rollback for `process.priority`, `process.affinity`, `power.scheme`; CLI optimize routes privileged keys through `AgentBackedPlatform` when the agent is connected; clear message when agent is not running; CLI `kraftverk agent serve|status`; doctor reports agent OK/FAIL; Windows elevation hint via token check
- **Search plugins**: ε-greedy multi-armed bandit and Bayesian GP+EI strategies selectable via `optimize --strategy`
- **Tauri desktop packaging**: optional `tauri-app` feature + `tauri.conf.json` + icon assets; default remains CI-safe `web-server` axum UI

### Changed
- **License**: replaced MIT with proprietary **All Rights Reserved** terms (`LicenseRef-Proprietary`). Copyright holder: **theworker02**. View/use as published; no modification or redistribution of modified versions without written permission. See [LICENSE](LICENSE).
- README / site / docs: professional presence (Swedish etymology, architecture diagram, badges, FUNDING.yml, CODE_OF_CONDUCT).
- Cargo workspace `license` set to `LicenseRef-Proprietary`; authors attributed to `theworker02`.
- Docs no longer describe agent/GPU as scaffold for current mainline.
- CI docs: clarify mock-platform / `KRAFTVERK_MOCK_PLATFORM` vs production `amd-only-v1` gate; branch-protection note for required checks on `main`.

### Fixed
- Tauri desktop icon assets for Windows packaging CI
- Remaining workspace clippy failures; Linux sensor build on stable Rust

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
