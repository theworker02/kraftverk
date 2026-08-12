# Security

## Threat model

| Asset | Risk | Mitigation |
|-------|------|------------|
| User files | Accidental overwrite by benches | Scratch-dir only + path refuse list |
| System config | Bad tunables | Tiny allow-list; rollback; recovery journal |
| Privacy | Fingerprint leaking identity | Hash hostname; no MAC/serial/username |
| Privilege abuse | Agent misuse | Authenticated local IPC only; no arbitrary shell; AMD hardware gate |

## Trust boundaries

```
[User] → kraftverk CLI (user privileges)
              │
              ├─ AMD-only hardware gate (amd-only-v1) before optimize/bench
              ├─ in-process safe tunables (bench.* / best-effort priority+affinity)
              │
              └─ authenticated local IPC → kraftverk-agent
                   (named pipe on Windows / Unix socket on Linux;
                    shared token; allow-list: process.priority, process.affinity, power.scheme)
```

Start the agent with `kraftverk agent serve` (elevated on Windows when changing power schemes). `kraftverk agent status` and `kraftverk doctor` report connectivity. The agent re-validates `amd-only-v1` on startup and before sensitive apply/rollback.

## Hardware policy

Unsupported architecture/vendor combinations refuse optimize, benchmark, SDK open, and privileged applies. Inspect-only surfaces (`compatibility`, `hardware`, `amd`) remain available to explain why a machine is blocked. See `docs/hardware-support.md`.


## Data at rest

Experiment DB lives under the platform project data directory (`directories` crate). Treat it as local performance history, not secrets — still avoid sharing raw machine reports if policy requires.
