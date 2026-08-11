#!/usr/bin/env bash
# Generate GitHub Release notes from CHANGELOG.md + install/hardware/license boilerplate.
# Usage: generate-release-notes.sh <tag> [output-file]
# Example: generate-release-notes.sh v0.2.2 /tmp/notes.md
set -euo pipefail

TAG="${1:?tag required (e.g. v0.2.2)}"
OUT="${2:-/tmp/kraftverk-release-notes.md}"
VERSION="${TAG#v}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CHANGELOG="$ROOT/CHANGELOG.md"

if [[ ! -f "$CHANGELOG" ]]; then
  echo "CHANGELOG.md not found at $CHANGELOG" >&2
  exit 1
fi

# Extract "## X.Y.Z — ..." section (until next ## heading)
SECTION="$(awk -v ver="$VERSION" '
  $0 ~ "^## " ver "([ .—–-]|$)" {capture=1}
  capture && NR>1 && /^## / {exit}
  capture {print}
' "$CHANGELOG")"

if [[ -z "${SECTION// }" ]]; then
  SECTION="## ${VERSION}

See [CHANGELOG.md](https://github.com/theworker02/kraftverk/blob/${TAG}/CHANGELOG.md) for details.
"
fi

WIN_ZIP="kraftverk-${TAG}-x86_64-pc-windows-msvc.zip"
LIN_TGZ="kraftverk-${TAG}-x86_64-unknown-linux-gnu.tar.gz"
BASE="https://github.com/theworker02/kraftverk/releases/download/${TAG}"

cat > "$OUT" <<EOF
## Highlights

Release **${TAG}** of Kraftverk — evidence-driven systems performance platform for AMD x86_64 hosts.

${SECTION}

## What's included

- **CLI** — \`kraftverk\` / \`kraftverk.exe\`
- **Desktop** — \`kraftverk-desktop\` / \`kraftverk-desktop.exe\` (local web UI)
- **AMD gate** — policy \`amd-only-v1\` (CPUID + PCI); exit codes 20–25
- Archives bundle \`README.md\`, \`LICENSE\`, and \`CHANGELOG.md\`

Confirm feature scope against the changelog section above (do not assume Tauri, GPU benches, or a finished privileged agent unless listed).

## Install

### Windows (x86_64)

1. Download [\`${WIN_ZIP}\`](${BASE}/${WIN_ZIP})
2. Verify against [\`${WIN_ZIP}.sha256\`](${BASE}/${WIN_ZIP}.sha256):

\`\`\`powershell
Get-FileHash .\\${WIN_ZIP} -Algorithm SHA256
Get-Content .\\${WIN_ZIP}.sha256
\`\`\`

3. Extract and run:

\`\`\`powershell
.\\kraftverk.exe compatibility
.\\kraftverk-desktop.exe
\`\`\`

### Linux (x86_64)

1. Download [\`${LIN_TGZ}\`](${BASE}/${LIN_TGZ})
2. Verify:

\`\`\`bash
sha256sum -c ${LIN_TGZ}.sha256
\`\`\`

3. Extract and run:

\`\`\`bash
tar xzf ${LIN_TGZ}
cd kraftverk-${TAG}-x86_64-unknown-linux-gnu
chmod +x kraftverk kraftverk-desktop
./kraftverk compatibility
./kraftverk-desktop
\`\`\`

### Build from source

\`\`\`bash
cargo build --release -p kraftverk-cli
cargo build --release -p kraftverk-desktop
\`\`\`

## Quick start

\`\`\`bash
kraftverk compatibility
kraftverk hardware
kraftverk inspect
kraftverk baseline
kraftverk optimize --mode safe --goal balanced
kraftverk status
kraftverk report --format html
kraftverk restore --baseline
\`\`\`

## Hardware requirements

| Check | Rule |
|-------|------|
| Architecture | **x86 / x86_64** only |
| CPU | **AMD** (\`AuthenticAMD\`) |
| GPU | None OK; if present, **AMD only** (PCI \`0x1002\`) |
| Blocked | Intel CPU; NVIDIA / Intel GPU; mixed / unknown |

Exit codes **20–25**. No production \`--force\` bypass.

- Docs: https://theworker02.github.io/kraftverk/docs/hardware-support.html
- Source: https://github.com/theworker02/kraftverk/blob/${TAG}/docs/hardware-support.md

## License

**Proprietary — All Rights Reserved.** Copyright © theworker02.

View/use as published. **No modification or redistribution of modified versions without written permission.**

- https://github.com/theworker02/kraftverk/blob/${TAG}/LICENSE

## Documentation

- Site: https://theworker02.github.io/kraftverk/
- README: https://github.com/theworker02/kraftverk/blob/${TAG}/README.md
- Safety: https://github.com/theworker02/kraftverk/blob/${TAG}/docs/safety.md
- Changelog: https://github.com/theworker02/kraftverk/blob/${TAG}/CHANGELOG.md

## Known limitations

See the changelog section for this version. Typical honest limits (confirm per tag):

- GPU benchmarks unsupported
- Portable temperature/power sensors unavailable
- Privileged agent may still be scaffold
- Desktop is local web UI unless a later tag ships Tauri packaging

## Checksums

Per-archive SHA-256 sidecars are attached to the release:

- \`${WIN_ZIP}.sha256\`
- \`${LIN_TGZ}.sha256\`

EOF

echo "Wrote $OUT"
