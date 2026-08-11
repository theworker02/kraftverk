# Platforms

## Supported (amd-only-v1)

Kraftverk only runs on **x86 / x86_64 hosts with an AMD CPU**. See [hardware-support.md](hardware-support.md) for the full policy, exit codes, and detection details.

| OS | Inspect | KraftBench | Safe optimize |
|----|---------|------------|---------------|
| Windows x86_64 (AMD CPU) | Yes | Yes | Yes (priority/affinity via Win32) |
| Linux x86_64 (AMD CPU) | Yes | Yes | Yes (nice / sched_setaffinity best-effort) |
| Intel CPU / NVIDIA or Intel GPU | Blocked (exit 21–25) | Blocked | Blocked |
| macOS / ARM / other arches | Not a release target (`compile_error!` on non-x86) | — | — |

## Capability honesty

`kraftverk inspect` / `kraftverk capabilities` list platform capabilities with `supported` / `partial` / `unsupported` / `requires_privilege`.

Examples unsupported in 0.2+: `gpu.clock`, `storage.trim`, `registry.tweaks`.
`power.scheme` requires the privileged agent (`kraftverk agent serve`; elevate on Windows).
GPU benches are real when AMD Vulkan is present; otherwise reported Unsupported.

## Fingerprint

Stable id from OS family/version, arch, CPU brand/cores, memory GiB bucket, hashed hostname, plus eligibility fields (`hardware_policy`, architecture, CPU vendor, compatibility, GPU vendors) — suitable for correlating experiments without unnecessary PII.

## Disclaimer

Kraftverk is an independent project and does not claim affiliation with or endorsement by AMD.
