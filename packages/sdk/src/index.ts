export class KraftverkApiError extends Error {
  readonly status: number;
  readonly payload?: unknown;

  constructor(message: string, status: number, payload?: unknown) {
    super(message);
    this.name = "KraftverkApiError";
    this.status = status;
    this.payload = payload;
  }
}

export interface KraftverkClientOptions {
  baseUrl?: string;
  fetchImpl?: typeof fetch;
}

export interface EligibilityResponse {
  ok: boolean;
  hardware_policy: string;
  supported: boolean;
  eligibility: unknown;
  startup_eligibility?: unknown;
  exit_code?: number | null;
}

export interface OverviewResponse {
  ok: boolean;
  blocked?: boolean;
  version?: string;
  hardware_policy: string;
  eligibility: unknown;
  fingerprint?: string;
  os?: string;
  baseline_id?: string | null;
  baseline_score?: number | null;
  accepted_id?: string | null;
  history_count?: number;
  philosophy?: string;
  error?: string;
  exit_code?: number | null;
}

export interface StatusResponse {
  ok: boolean;
  db?: string | null;
  fingerprint: string;
  active_candidate: unknown;
  hardware_policy: string;
  eligibility: unknown;
  agent: unknown;
  agent_connected: boolean;
}

export interface TelemetryResponse {
  ok: boolean;
  snapshot: unknown;
}

export interface HistoryResponse {
  ok: boolean;
  experiments?: unknown[];
  error?: string;
  blocked?: boolean;
}

export interface BenchmarkResponse {
  ok: boolean;
  hardware_policy?: string;
  raw_mean?: unknown;
  measurements?: unknown;
  note?: string;
  error?: string;
  blocked?: boolean;
}

function trimSlash(url: string): string {
  return url.replace(/\/+$/, "");
}

/**
 * Typed client for the Kraftverk desktop instrument (`127.0.0.1:47821`).
 */
export class KraftverkClient {
  readonly baseUrl: string;
  private readonly fetchImpl: typeof fetch;

  constructor(opts: KraftverkClientOptions = {}) {
    this.baseUrl = trimSlash(opts.baseUrl ?? "http://127.0.0.1:47821");
    this.fetchImpl = opts.fetchImpl ?? fetch;
  }

  eligibility(): Promise<EligibilityResponse> {
    return this.get("/api/eligibility");
  }

  overview(): Promise<OverviewResponse> {
    return this.get("/api/overview");
  }

  history(limit = 20): Promise<HistoryResponse> {
    const query = new URLSearchParams({ limit: String(limit) });
    return this.get(`/api/history?${query}`);
  }

  telemetry(): Promise<TelemetryResponse> {
    return this.get("/api/telemetry");
  }

  status(): Promise<StatusResponse> {
    return this.get("/api/status");
  }

  benchmark(): Promise<BenchmarkResponse> {
    return this.get("/api/benchmark");
  }

  private async get<T>(path: string): Promise<T> {
    const res = await this.fetchImpl(`${this.baseUrl}${path}`);
    const text = await res.text();
    const payload = text ? (JSON.parse(text) as unknown) : undefined;
    if (!res.ok) {
      throw new KraftverkApiError(`HTTP ${res.status} ${path}`, res.status, payload);
    }
    return payload as T;
  }
}

export default KraftverkClient;
