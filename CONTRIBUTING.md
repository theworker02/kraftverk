# Contributing

Thanks for helping Kraftverk stay honest.

## Rules of the road

1. **Evidence only** — never invent telemetry, benchmark scores, or “boost” claims.
2. Prefer measured improvements or better instrumentation for decisions.
3. Keep the first-party crate count **≤ 10** (merge modules instead of adding crates).
4. Safe changes must be reversible and journaled.
5. Use conventional commits: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`.

## Workflow

```bash
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
```

Open a focused PR with a short summary and test plan.

## Security

See [SECURITY.md](SECURITY.md).
