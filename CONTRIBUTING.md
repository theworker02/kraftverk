# Contributing

Thanks for helping Kraftverk stay honest and evidence-driven.

## License reminder

Kraftverk is **proprietary** software owned by **theworker02**. It is **not** MIT/Apache-licensed. See [LICENSE](LICENSE).

By submitting a contribution (PR, patch, or suggestion), you grant the Copyright Holder rights to include it under the project’s proprietary terms, as described in the LICENSE. Opening a PR does **not** re-license the project or authorize third parties to redistribute modified versions.

## Rules of the road

1. **Evidence only** — never invent telemetry, benchmark scores, or “boost” claims.
2. Prefer measured improvements or better instrumentation for decisions.
3. Keep the first-party crate count **≤ 10** (merge modules instead of adding crates).
4. Safe changes must be reversible and journaled.
5. Preserve **`amd-only-v1`** hardware enforcement — do not weaken the gate.
6. Use conventional commits: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`.
7. Follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Workflow

```bash
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace --features kraftverk-system/mock-platform
```

Open a focused PR with a short summary and test plan. Discuss larger design changes in an issue first when possible.

## Security

See [SECURITY.md](SECURITY.md).
