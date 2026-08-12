# Development

## Prerequisites

- Rust 1.75+ (CI uses stable)
- Windows or Linux recommended for full tuning; macOS for inspect/bench only (no parity claim)

## Build

```bash
cargo build --workspace
cargo build --release -p kraftverk-cli
cargo build --release -p kraftverk-desktop
cargo build --release -p kraftverk-agent
```

GPU benches are behind the `kraftverk-bench` `gpu` feature (default on). Disable with `--no-default-features` on that crate if needed.

## Test

```bash
cargo test --workspace --features kraftverk-system/mock-platform
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

### CI vs production hardware gate

GitHub Actions runners are typically Intel and may lack AMD GPUs. CI stays green without
weakening `amd-only-v1` for users:

- **Tests** enable `kraftverk-system/mock-platform` so eligibility cases use injected facts
  (`MockPlatform` / `evaluate_from_facts`). This is a **dev/test Cargo feature**, not a
  release CLI flag or env-var bypass.
- **Clippy / release build** in CI compile the real gate (no mock feature).
- **Do not** add `--force`, `--ignore-hardware`, or `KRAFTVERK_SKIP_HARDWARE_GATE` to
  production binaries.
- GPU benches skip cleanly when Vulkan/AMD is unavailable.

Details: `docs/hardware-support.md`, `.github/workflows/ci.yml`.

## Privileged agent

```bash
# Elevated terminal recommended for power.scheme changes
cargo run -p kraftverk-agent
# or
cargo run -p kraftverk-cli -- agent serve

cargo run -p kraftverk-cli -- agent status
cargo run -p kraftverk-cli -- doctor
```

IPC is local-only (Windows named pipe `\\.\pipe\kraftverk-agent`, Linux Unix socket under the Kraftverk data dir). Auth token is written under the app data `agent/auth.token`.

## Search strategies

```bash
kraftverk optimize --strategy hill-climb
kraftverk optimize --strategy epsilon-greedy
kraftverk optimize --strategy bayesian
```

## Desktop

### Default (local web UI — CI-safe)

```bash
cargo run -p kraftverk-desktop
# opens http://127.0.0.1:47821/
```

### Tauri native shell

Requires platform WebView dependencies (WebView2 on Windows, webkit2gtk on Linux).

```bash
# One-shot check / run (feature not default — keeps workspace CI green)
cargo run -p kraftverk-desktop --no-default-features --features tauri-app

# Or with the Tauri CLI (install: cargo install tauri-cli --version "^2")
cd crates/kraftverk-desktop
cargo tauri dev
cargo tauri build
```

Icons resolve from `assets/icon-*.svg`. If `cargo tauri` / WebView libs are missing on a runner, use the default `web-server` feature only — do not fail the whole workspace job.

## Dev simulation

```bash
cargo run -p kraftverk-cli --features dev-simulate -- dev simulate-machine quiet
```

## Crate layout (≤10)

See [crate-consolidation.md](crate-consolidation.md).
