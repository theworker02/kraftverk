# API stability policy

The supported integration surface is **`kraftverk-sdk`**.

## Guarantees (0.2.x)

- Public re-exports from `kraftverk-sdk` follow semantic versioning.
- Breaking changes to SDK exports require a minor bump while pre-1.0, documented in `CHANGELOG.md`.
- CLI `--json` shapes for core commands (`inspect`, `baseline`, `benchmark`, `optimize`, `status`, `history`) are treated as soft-stable: fields may be added, but existing fields are not renamed without a version note.

## Non-guarantees

- Internal crates (`kraftverk-system` module layout, private helpers) may change.
- Desktop local HTTP routes may evolve; prefer CLI/SDK for automation.
- GPU / privileged agent APIs remain experimental until backends exist.

## Versioning

Package version is `0.2.0` for the Expansive Platform Phase. Milestone 1 was `0.1.x`.
