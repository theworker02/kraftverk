# Telemetry

Milestone 1 telemetry is **local** and **minimal**.

## Collected

- Timestamp
- Process-visible CPU usage % (sysinfo)
- Memory used/total
- Coarse load hint (`low` / `moderate` / `high`)

## Not collected (explicitly unsupported)

- Package temperature
- Fan RPM
- Power draw (watts)
- GPU utilization / clocks
- Network identities

Snapshots are attached to experiments for context. They are **never** used to invent benchmark scores.

Future milestones may add optional sensor backends behind the same honesty rules.
