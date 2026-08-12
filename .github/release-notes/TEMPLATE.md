# Release notes template (for humans + CI)

Future tag releases should ship notes with these sections. The Release workflow
prefers a curated `.github/release-notes/<tag>.md` when present; otherwise it
builds a body from the matching `CHANGELOG.md` version section plus the
boilerplate below.

## Required sections

1. **Highlights** — 2–4 sentences; version-accurate
2. **What's included** — CLI / desktop / benches / optimizer / AMD gate / SDK as applicable
3. **Install** — Windows zip + `.sha256`, Linux tar.gz + `.sha256`, extract/run commands
4. **Quick start** — real binary names from assets (`kraftverk` / `kraftverk.exe`)
5. **Hardware requirements** — AMD x86_64 only; exit codes 20–25; link to hardware-support docs
6. **License** — Proprietary All Rights Reserved © theworker02; link to LICENSE
7. **Documentation** — Pages site, README, SAFETY, CHANGELOG
8. **Known limitations** — version-accurate; do not claim Tauri/GPU/agent if absent
9. **Checksums** — point at `*.sha256` sidecar assets (not a combined SHA256SUMS unless attached)

## Asset naming (workflow)

- `kraftverk-<tag>-x86_64-pc-windows-msvc.zip` (+ `.sha256`)
- `kraftverk-<tag>-x86_64-unknown-linux-gnu.tar.gz` (+ `.sha256`)
- Loose: `kraftverk` / `kraftverk.exe`, `kraftverk-desktop` / `kraftverk-desktop.exe`
- Docs in archive: `README.md`, `LICENSE`, `CHANGELOG.md`
