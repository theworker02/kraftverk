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

## GPU benches (KraftBench)

When an AMD Vulkan device is present (`kraftverk-bench` feature `gpu`, default on Win/Linux x86_64), KraftBench runs real GPU workloads via `ash`:

- Buffer copy bandwidth (`gpu.mem_copy_bandwidth`)
- Compute throughput (`gpu.compute_throughput`)
- XOR reduction / hash-style kernel (`gpu.reduction_hash`)

Device selection prefers discrete AMD (PCI vendor `0x1002`). Results flow into the Kraft Index GPU category when measurements exist. If Vulkan or an AMD adapter is missing, the suite records an honest `Unsupported` reason and does **not** fabricate scores.

## Not collected

- Fan RPM
- GPU clocks / utilization (vendor SDK)
- Network identities
- Undocumented MSR poking

Snapshots attach to experiments for context only.
