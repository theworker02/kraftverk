# Security Policy

## Reporting

Please open a private GitHub security advisory or email the maintainers. Do not file public issues for privilege-escalation paths in the agent IPC design.

## Scope

- Kraftverk must not invent telemetry or hide failed rollbacks.
- Privileged operations belong behind an authenticated local agent (scaffold in 0.2).
- Storage benchmarks must never write to user Documents/Desktop paths.
