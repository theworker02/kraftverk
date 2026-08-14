# `@theworker02/kraftverk-sdk`

Typed control client for the Kraftverk desktop HTTP API on `http://127.0.0.1:47821`.

```ts
import { KraftverkClient } from "@theworker02/kraftverk-sdk";

const api = new KraftverkClient();
const elig = await api.eligibility();
if (!elig.supported) throw new Error("host is not eligible");

const overview = await api.overview();
const history = await api.history(50);
const probe = await api.benchmark();
```

Start the desktop instrument first (`cargo run -p kraftverk-desktop`).

## Methods

| Method | Endpoint |
| --- | --- |
| `eligibility()` | `GET /api/eligibility` |
| `overview()` | `GET /api/overview` |
| `history(limit?)` | `GET /api/history` |
| `telemetry()` | `GET /api/telemetry` |
| `status()` | `GET /api/status` |
| `benchmark()` | `GET /api/benchmark` |
