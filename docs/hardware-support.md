# Hardware support (amd-only-v1)

Kraftverk is an **AMD-exclusive** performance engineering platform. This is a hard execution gate across CLI, desktop, SDK, agent, and optimizer — not a cosmetic preference.

**Independent product disclaimer:** Kraftverk is not affiliated with, endorsed by, or impersonating Advanced Micro Devices, Inc. (AMD). “AMD” in this document refers to CPU/GPU vendor identity detected on the host (CPUID / PCI), not a partnership claim.

## Supported configuration

| Check | Rule |
|-------|------|
| Architecture | `x86` or `x86_64` only (`compile_error!` on other targets) |
| CPU vendor | AMD (`AuthenticAMD`) via CPUID |
| GPU | None → allowed; if present, **all** must be AMD (PCI vendor `0x1002`) |
| Blocked | Intel CPU; NVIDIA GPU (`0x10DE`); Intel GPU (`0x8086`); mixed AMD+NVIDIA; unknown CPU/GPU vendors |

Examples that **PASS**: AMD CPU + no GPU; AMD CPU + AMD GPU(s).  
Examples that **FAIL**: Intel CPU; AMD CPU + NVIDIA; AMD+NVIDIA multi-GPU; ARM/aarch64 builds.

Policy id persisted on experiments: `hardware_policy = "amd-only-v1"`.

## Exit codes (20–25)

| Code | Meaning |
|-----:|---------|
| 20 | Unsupported architecture |
| 21 | Intel CPU detected |
| 22 | NVIDIA GPU detected |
| 23 | Intel GPU detected |
| 24 | Unknown CPU vendor |
| 25 | Unsupported combination (mixed / unknown GPU / other) |

There is **no** production `--force` bypass. Tests may use the `mock-platform` feature to inject hardware facts.

## Detection

- **CPU:** CPUID leaf 0 vendor string; Linux `/proc/cpuinfo` and sysinfo fallbacks.
- **GPU:** Windows PCI Enum registry (`VEN_xxxx`, Display class); Linux `/sys/bus/pci` class `0x03` — **no `lspci` dependency**.
- **Hot-plug:** optimizer / session guards re-check; NVIDIA appearance mid-session → stop, restore managed config, block.

## Product surfaces

| Surface | Behavior |
|---------|----------|
| CLI | Shared pre-dispatch gate; `kraftverk compatibility` / `kraftverk hardware` / `kraftverk amd …` are inspect-only |
| Desktop | Splash/blocker + `/api/eligibility`; gated APIs return 403 when blocked |
| SDK | `UnsupportedHardware`, `Kraftverk::open_default()`, `require_supported_hardware()` |
| Agent | `PrivilegedAgent::start()` + re-check before apply/rollback ops |
| Optimizer | Session guard revalidation each search iteration |

## CLI (inspect-only)

```bash
kraftverk compatibility   # policy status + reasons (no gate)
kraftverk hardware        # CPUID + PCI inventory (no gate)
kraftverk amd cpu         # Ryzen topology hints (honest / unset when unknown)
kraftverk amd gpu         # AMD GPU list (PCI 0x1002)
```

## Release targets

Official binaries: **x86_64 Windows** and **x86_64 Linux** only.
