# Safety

## Non-goals

Kraftverk will not:

- Delete user files or “clean” temp folders outside its scratch dir
- Apply irreversible registry/firmware tweaks casually
- Disable security features
- Fake improvements
- Run optimize/benchmark on non-AMD or mixed NVIDIA systems (policy `amd-only-v1`)

## Hardware eligibility boundary

- Central gate in `kraftverk-system` eligibility subsystem
- CLI / desktop / SDK / agent enforce independently
- Hot-plug NVIDIA mid-session → stop experiments, restore managed config, block further execution
- No production `--force` / `--ignore-hardware` bypass

## Guardrails

- Safe mode parameter allow-list only
- Apply → verify → rollback with journal
- Storage I/O confined to marked scratch directories
- Optimizer search always rolls back; only validated accepts remain
- `kraftverk restore` clears active accepted config

## Thermal / correctness

- Temperature/power sensors unavailable portably (marked unsupported)
- Checksums on many benches detect obvious incorrect results
- High variance → `UNSTABLE_RESULT` / stability FAIL → reject
