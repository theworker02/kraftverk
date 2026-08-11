# Security Policy

## Reporting

Please open a **private** GitHub security advisory on
[theworker02/kraftverk](https://github.com/theworker02/kraftverk), or contact
[@theworker02](https://github.com/theworker02) directly.

Do **not** file public issues for privilege-escalation paths in the agent IPC
design or other sensitive vulnerabilities.

## Scope

- Kraftverk must not invent telemetry or hide failed rollbacks.
- Privileged operations belong behind an authenticated local agent (named pipe / Unix socket + token handshake).
- Storage benchmarks must never write to user Documents/Desktop paths.
- Hardware eligibility (`amd-only-v1`) is a safety and product boundary — bypasses are out of scope for “fixes.”

## Supported versions

Security fixes are considered for the latest release on `main` and the most recent tagged release. Older tags may not receive backports.

## License

Kraftverk is proprietary (All Rights Reserved). See [LICENSE](LICENSE). Security research against published builds is welcome; redistribution of modified builds is not.
