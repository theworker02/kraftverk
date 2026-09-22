# Acquisition Brief â€” â†’ http://127.0.0.1:47821/

**Date:** 2026-09-22  
**Repository:** https://github.com/theworker02/kraftverk  
**Default branch:** `main`  
**Primary language:** Rust  
**Status:** Diligence briefing only. **No acquisition has occurred** by virtue of this file.  
**License:** Proprietary â€” sale, written commercial license, or completed asset transfer required (see root `LICENSE`).  
**Valuation:** Not stated.  
**Contact:** GitHub [@theworker02](https://github.com/theworker02) Â· [thanks.dev/u/gh/theworker02](https://thanks.dev/u/gh/theworker02)

> Cloning or forking this repository does **not** grant production, redistribution, SaaS, OEM, or commercial rights.

---

## 1. Executive thesis

<img src="assets/wordmark.svg" alt="Kraftverk" width="420" /> <img src="assets/swedish-flag.svg" alt="" width="28" height="18" /> <strong>Evidence-driven systems performance platform</strong><br />

**Why a buyer cares:** â†’ http://127.0.0.1:47821/ packages transferable product IP â€” source, docs, in-repo brand assets, and a diligence room under `docs/acquisition/` â€” under a clear proprietary posture so diligence can proceed without mistaking the repo for open source.

---

## 2. Product snapshot

| Item | Detail |
|------|--------|
| Product | â†’ http://127.0.0.1:47821/ |
| Repo | `theworker02/kraftverk` |
| Language | Rust |
| Open source? | **No** â€” proprietary |
| Rightsholder | theworker02 |
| Diligence pack | `docs/acquisition/` |

### Capability highlights (from current materials)

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

---

## 3. Problem / opportunity

Teams evaluating â†’ http://127.0.0.1:47821/ typically need either (a) a commercial right to run or embed it, or (b) outright ownership of the Product IP for strategic build-out. Public GitHub visibility without a proprietary license creates false assumptions about free production use. This brief and the linked data room make the commercial path explicit.

---

## 4. What ships today

Honest maturity: treat repository contents, README claims, tests, and release tags as the source of truth. Do not assume production customers, ARR, filed patents, or SLAs unless separately evidenced in diligence.

Typical transferable surfaces:

- Source tree and build/test scripts present in-repo
- Documentation and design notes
- Acquisition / diligence markdown under `docs/acquisition/`
- Branding assets committed to the repository (if any)

---

## 5. Demo / evaluation path (buyer)

Minimal path (no secrets required unless README says otherwise):

```
```mermaid
flowchart LR
  M[Measure] --> E[Experiment]
  E --> V[Validate]
  V --> K{Keep or Revert}
  K -->|Keep| L[Learn]
  K -->|Revert| L
  L -->|Repeat| M
```
```bash
kraftverk compatibility
kraftverk hardware
kraftverk amd cpu
kraftverk amd gpu
```
```bash
kraftverk optimize --mode safe --goal balanced
kraftverk optimize --mode balanced --goal compile
kraftverk optimize --mode aggressive --goal throughput
```
```bash
kraftverk optimize --strategy hill-climb
kraftverk optimize --strategy epsilon-greedy
kraftverk optimize --strategy bayesian
```
```bash
kraftverk report --format html
kraftverk report --format json -o report.json
kraftverk receipt <experiment-id>
kraftverk receipt --verify path/to.kraft-receipt.json
kraftverk explain <id>
kraftverk compare <a> <b>
kraftverk analyze recent
```
```
crates/
â”œâ”€â”€ kraftverk-core/       # Domain models, stats, Kraft Index, goals/constraints
â”œâ”€â”€ kraftverk-system/     # Platform + hardware eligibility + sensors/telemetry
â”œâ”€â”€ kraftverk-bench/      # KraftBench workloads (CPUâ€¦ + optional AMD Vulkan GPU)
â”œâ”€â”€ kraftverk-optimizer/  # Search strategies, profiles, objectives
```

Extended evaluation: `docs/acquisition/BUYER_EVALUATION.md`. Written NDA / evaluation grants may be required for private materials.

---

## 6. What a transaction typically includes

Subject to definitive schedules:

| Included (typical) | Excluded (typical) |
|--------------------|--------------------|
| Repo materials + asserted original IP | Seller personal accounts / unrelated repos |
| Docs + diligence room at closing | Third-party dependency source under separate licenses |
| In-repo brand marks as assigned | Secrets without rotation plan |
| Know-how captured in docs | Fabricated revenue, user, or adoption metrics |

---

## 7. Suggested deal structures

| Structure | When it fits |
|-----------|--------------|
| Non-exclusive commercial license | Deploy/run under seat or environment terms |
| Exclusive field-of-use license | Buyer wants exclusivity; seller may retain entity |
| Asset / IP assignment | Buyer wants ownership of Materials outright |
| OEM / redistribution | Separate agreement â€” not implied here |

Commercial terms (price, earnouts, escrow) are negotiated under NDA with counsel.

---

## 8. Buyer diligence checklist

- [ ] Confirm Rightsholder identity and authority to sell/license
- [ ] Inventory Materials (`docs/acquisition/ASSET_INVENTORY.md`)
- [ ] Review IP posture (`IP_PROVENANCE.md`) and dependencies (`DEPENDENCY_INVENTORY.md`)
- [ ] Run evaluation script (`BUYER_EVALUATION.md`)
- [ ] Review risks (`RISK_REGISTER.md`)
- [ ] Agree transfer scope (`TRANSFER_MANIFEST.md`) and handoff (`HANDOFF_CHECKLIST.md`)
- [ ] Supersede root `LICENSE` at closing via definitive agreement

---

## 9. Related documents

| Document | Purpose |
|----------|---------|
| `LICENSE` | Proprietary â€” no default grant |
| `docs/acquisition/README.md` | Data-room index |
| `docs/acquisition/EXECUTIVE_SUMMARY.md` | One-page thesis |
| `README.md` | Product overview |
| `SECURITY.md` | Vulnerability reporting |
| `COMMERCIAL.md` | Licensing contact path |
| `.github/FUNDING.yml` | Sponsors / thanks.dev |

---

## 10. Disclaimer

This package is informational and **does not** create a binding offer, grant of rights, or investment advice. Engage counsel for any transaction.

---

*Document version: 2.0.0 / 2026-09-22 Â· Classification: acquisition briefing*
