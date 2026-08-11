# Development

## Prerequisites

- Rust 1.75+ (CI uses stable)
- Windows or Linux recommended for full tuning; macOS for inspect/bench

## Build

```bash
cargo build --workspace
cargo build --release -p kraftverk-cli
cargo build --release -p kraftverk-desktop
```

## Test

```bash
cargo test --workspace
```

## Dev simulation

```bash
cargo run -p kraftverk-cli --features dev-simulate -- dev simulate-machine quiet
```

## Crate layout (≤10)

See [crate-consolidation.md](crate-consolidation.md).

## Desktop

```bash
cargo run -p kraftverk-desktop
```

Opens `http://127.0.0.1:47821/` against the same local DB as the CLI.
