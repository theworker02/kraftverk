# Telemetry

Telemetry is **local**, **minimal**, and **never** invents temperatures, watts, or benchmark scores.

## Collected (all platforms)

- Timestamp
- Process-visible CPU usage % (`sysinfo`)
- Memory used/total
- Process count (noise model)
- Coarse load hint (`low` / `moderate` / `high`)
- Environmental noise estimate (heuristic contention proxy)

## Temperature / power (OS-backed when present)

| Source | Linux | Windows |
|--------|-------|---------|
| CPU package / die temp | `/sys/class/hwmon` (`k10temp`, `zenpower`, `coretemp`, …) | ACPI thermal zones via WMI `MSAcpi_ThermalZoneTemperature` (OEM-dependent; often coarse or absent) |
| AMD GPU temp | `amdgpu` hwmon `temp*_input` when present | Not linked (no ADL); unset unless OS exposes a zone |
| Package power | RAPL `/sys/class/powercap/*/energy_uj` delta → watts; hwmon `power*_input` | Not available via portable free APIs without vendor SDKs — remains unset |

When a reading is unavailable, snapshots leave `temp_c` / `power_w` as `null` and note the reason. Optimizer `--max-temp` / `--max-power` only enforce limits when readings exist; otherwise they are recorded as unchecked.

Inspect `kraftverk doctor` for sensor availability. See also `kraftverk inspect`.

## Not collected

- Fan RPM
- GPU clocks / utilization (vendor SDK)
- Network identities
- Undocumented MSR poking

Snapshots attach to experiments for context only.
