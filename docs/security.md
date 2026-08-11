# Security

## Threat model (M1)

| Asset | Risk | Mitigation |
|-------|------|------------|
| User files | Accidental overwrite by benches | Scratch-dir only + path refuse list |
| System config | Bad tunables | Tiny allow-list; rollback; safe mode only |
| Privacy | Fingerprint leaking identity | Hash hostname; no MAC/serial/username |
| Privilege abuse | Future agent misuse | Agent not operational; IPC design requires auth |

## Trust boundaries

```
[User] → kraftverk CLI (user privileges)
              │
              ├─ AMD-only hardware gate (amd-only-v1) before optimize/bench
              ├─ in-process safe tunables
              │
              └─ (future) authenticated IPC → kraftverk-agent (elevated; also hardware-gated)
```

The agent scaffold (`kraftverk-agent`) validates hardware on `PrivilegedAgent::start()` and re-checks before sensitive ops. It does not yet listen on a production socket.

## Hardware policy

Unsupported architecture/vendor combinations refuse optimize, benchmark, SDK open, and privileged applies. Inspect-only surfaces (`compatibility`, `hardware`, `amd`) remain available to explain why a machine is blocked. See `docs/hardware-support.md`.


## Data at rest

Experiment DB lives under the platform project data directory (`directories` crate). Treat it as local performance history, not secrets — still avoid sharing raw machine reports if policy requires.
