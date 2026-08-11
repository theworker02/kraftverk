# Optimizer

## Mode

Milestone 1 supports **`--mode safe` only**. Balanced/Aggressive are rejected with a clear error.

## Safe parameters (implemented)

| Key | Scope | Rollback |
|-----|-------|----------|
| `bench.worker_threads` | Process / KraftBench workers | Yes |
| `bench.rayon_threads` | Local rayon pool size for benches | Yes |
| `process.priority` | Current process (`normal` / `above_normal` / `high`) | Yes (best-effort) |
| `process.affinity` | Current process (`all` / `even` / `odd` / `first_half`) | Yes (best-effort) |

Not implemented (explicitly unsupported): GPU clocks, power plans, registry tweaks, TRIM/cleaners, other-process affinity.

## Search

- Strategy: deterministic **hill climbing** (`HillClimbStrategy`) with reproducible seed
- Neighborhood: discrete thread counts, priority, affinity variants
- Stop conditions: max experiments, time budget, plateau limit, neighborhood exhausted, instability

## Lifecycle per candidate

1. Apply via `ApplyGuard` (writes recovery journal)
2. Verify each change
3. Measure (warm-up + samples)
4. Compare to baseline index samples
5. **Rollback** during search
6. Provisional best → **validation** run with more samples
7. Accept + `commit` (keep applied) or reject + rollback

## Stability gate

Validation requires:

- Stability `PASS` (CoV ≤ threshold, checksums present, ≥2 samples)
- Comparison class improvement (`LIKELY_*` or `CONFIRMED_*`)

## Crash recovery

On launch, `RecoveryJournal` restores any interrupted apply.
