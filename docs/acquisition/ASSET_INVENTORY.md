# Asset inventory â€” â†’ http://127.0.0.1:47821/

## Repository surfaces

| Asset | Location / notes |
|-------|------------------|
| Source tree | Repository root / language packages |
| Tests | `test/`, `tests/`, CI workflows if present |
| Docs | `README.md`, `docs/` |
| Diligence room | `docs/acquisition/` |
| License / notices | `LICENSE`, transition notices if present |
| Funding | `.github/FUNDING.yml` |
| CI | `.github/workflows/` if present |
| Branding | logos/assets folders if present |

## Capability highlights

- Discovers hardware facts and enforces an **AMD-only** eligibility gate (`amd-only-v1`)
- Runs **KraftBench** â€” real, deterministic workloads with checksums where practical
- Builds a **Kraft Index** (baseline-normalized composite score)
- Searches **reversible** tunables (process-scoped workers, priority, affinity; privileged power schemes via agent)
- Accepts a candidate only after **statistical validation** against the baseline
- Persists experiments, sessions, receipts, and HTML/JSON reports in a local SQLite store
- Offers a CLI, optional privileged agent, and a desktop instrument (default local web UI; optional Tauri)
- **Allow-listed parameters only** for in-process safe tuning (`bench.worker_threads`, `bench.rayon_threads`, `process.priority`, `process.affinity`)
- **Apply â†’ verify â†’ rollback** with a recovery journal (`ApplyGuard`)
- Search iterations **roll back**; only validated accepts remain applied
- `kraftverk restore` / `kraftverk restore --baseline` clears active accepted config
- Crash recovery restores interrupted applies on launch

## Usually excluded

Seller personal accounts, unrelated repos, and unreissued registry tokens â€” unless listed in the definitive agreement.

*Updated: 2026-09-22*
