const views = {
  overview: renderOverview,
  benchmark: renderBenchmark,
  optimize: renderOptimize,
  experiments: renderExperiments,
  profiles: renderProfiles,
  telemetry: renderTelemetry,
  reports: renderReports,
  system: renderSystem,
  settings: renderSettings,
  safety: renderSafety,
};

const titleEl = document.getElementById("view-title");
const viewEl = document.getElementById("view");
const pill = document.getElementById("status-pill");
const hwBlock = document.getElementById("hw-block");
const appShell = document.getElementById("app-shell");
let hardwareBlocked = false;

document.querySelectorAll(".nav button").forEach((btn) => {
  btn.addEventListener("click", () => {
    if (hardwareBlocked) return;
    document.querySelectorAll(".nav button").forEach((b) => b.classList.remove("active"));
    btn.classList.add("active");
    const name = btn.dataset.view;
    titleEl.textContent = btn.textContent;
    views[name]();
  });
});

async function api(path) {
  // Tauri 2 shell (withGlobalTauri): invoke instead of fetch.
  const invoke = window.__TAURI__?.core?.invoke;
  if (typeof invoke === "function") {
    if (path.startsWith("/api/history")) {
      const u = new URL(path, "http://local");
      const limit = Number(u.searchParams.get("limit") || 20);
      return invoke("cmd_history", { limit });
    }
    const map = {
      "/api/eligibility": "cmd_eligibility",
      "/api/overview": "cmd_overview",
      "/api/status": "cmd_status",
      "/api/telemetry": "cmd_telemetry",
      "/api/benchmark": "cmd_benchmark",
    };
    const cmd = map[path];
    if (cmd) return invoke(cmd);
  }
  const res = await fetch(path);
  return res.json();
}

function metric(label, value) {
  return `<div class="metric"><div class="label">${label}</div><div class="value">${value ?? "—"}</div></div>`;
}

async function preflightHardware() {
  const data = await api("/api/eligibility");
  if (!data.supported) {
    hardwareBlocked = true;
    appShell.hidden = true;
    hwBlock.hidden = false;
    const el = data.eligibility || {};
    const reasons = (el.rejection_reasons || [])
      .map((r) => (typeof r === "string" ? r : JSON.stringify(r)))
      .join("\n");
    document.getElementById("hw-block-detail").textContent = [
      `Policy: ${data.hardware_policy || "amd-only-v1"}`,
      `Arch: ${el.architecture}`,
      `CPU vendor: ${el.cpu_vendor} (${el.cpu_vendor_raw || ""})`,
      `GPUs: ${(el.gpu_vendors || []).join(", ") || "none"}`,
      `Exit code: ${data.exit_code ?? "—"}`,
      "",
      data.error || "BLOCKED",
      reasons,
    ].join("\n");
    pill.textContent = "blocked";
    return false;
  }
  hardwareBlocked = false;
  hwBlock.hidden = true;
  appShell.hidden = false;
  return true;
}

async function renderOverview() {
  viewEl.innerHTML = `<p class="muted">Loading…</p>`;
  const data = await api("/api/overview");
  if (data.blocked) {
    await preflightHardware();
    return;
  }
  pill.textContent = data.ok
    ? `v${data.version} · ${data.hardware_policy || "amd-only-v1"}`
    : "error";
  viewEl.innerHTML = `
    <div class="card-free">
      <h2>Kraftverk</h2>
      <p>${data.philosophy}</p>
      <p class="muted">Hardware policy: <code>${data.hardware_policy || "amd-only-v1"}</code></p>
    </div>
    <div class="grid">
      ${metric("Baseline", data.baseline_score != null ? data.baseline_score.toFixed(0) : "none")}
      ${metric("History", data.history_count)}
      ${metric("OS", data.os)}
      ${metric("Fingerprint", (data.fingerprint || "").slice(0, 12) + "…")}
    </div>
    <p class="muted">Use the CLI for full optimize loops. This desktop shares the same SQLite store — no separate scores.</p>
  `;
}

async function renderBenchmark() {
  if (hardwareBlocked) return;
  viewEl.innerHTML = `
    <div class="card-free">
      <h2>KraftBench probe</h2>
      <p class="muted">Runs one real sample (no storage/scaling). Decision-quality baselines still belong in the CLI.</p>
      <button class="action" id="run-bench">Run probe</button>
      <div id="bench-out" style="margin-top:1rem"></div>
    </div>`;
  document.getElementById("run-bench").onclick = async () => {
    const out = document.getElementById("bench-out");
    out.innerHTML = `<p class="muted">Measuring…</p>`;
    const data = await api("/api/benchmark");
    if (!data.ok) {
      out.innerHTML = `<p>${data.error || "blocked"}</p>`;
      if (data.blocked) await preflightHardware();
      return;
    }
    const rows = (data.measurements || [])
      .map(
        (m) =>
          `<tr><td><code>${m.id}</code></td><td>${m.category}</td><td class="mono">${Number(
            m.raw_value
          ).toFixed(3)}</td><td>${m.unit}</td></tr>`
      )
      .join("");
    out.innerHTML = `
      <p>Raw composite sample: <span class="mono">${data.raw_mean}</span></p>
      <p class="muted">${data.note}</p>
      <table><thead><tr><th>ID</th><th>Cat</th><th>Raw</th><th>Unit</th></tr></thead><tbody>${rows}</tbody></table>`;
  };
}

function renderOptimize() {
  viewEl.innerHTML = `
    <div class="banner">Optimize loops are CLI-first for auditability. Desktop reflects session/history state.</div>
    <div class="card-free">
      <h2>Recommended flow</h2>
      <ul class="plain">
        <li><code>kraftverk compatibility</code></li>
        <li><code>kraftverk baseline</code></li>
        <li><code>kraftverk optimize --mode safe --goal balanced</code></li>
        <li><code>kraftverk status</code> · <code>kraftverk restore --baseline</code></li>
      </ul>
      <p class="muted">No Optimize bypass exists in normal builds. Hardware policy amd-only-v1 is mandatory.</p>
    </div>`;
}

async function renderExperiments() {
  viewEl.innerHTML = `<p class="muted">Loading experiments…</p>`;
  const data = await api("/api/history?limit=30");
  if (!data.ok) {
    viewEl.innerHTML = `<p>${data.error}</p>`;
    if (data.blocked) await preflightHardware();
    return;
  }
  const rows = (data.experiments || [])
    .map((e) => {
      const score = e.index_summary ? e.index_summary.mean.toFixed(1) : "—";
      return `<tr>
        <td><code>${String(e.id).slice(0, 8)}</code></td>
        <td>${e.kind}</td>
        <td>${e.decision}</td>
        <td class="mono">${score}</td>
        <td>${e.hardware_policy || ""}</td>
        <td>${e.decision_reason || ""}</td>
      </tr>`;
    })
    .join("");
  viewEl.innerHTML = `
    <table>
      <thead><tr><th>ID</th><th>Kind</th><th>Decision</th><th>Index</th><th>Policy</th><th>Reason</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No experiments yet.</td></tr>`}</tbody>
    </table>`;
}

function renderProfiles() {
  viewEl.innerHTML = `
    <div class="card-free">
      <h2>Profiles</h2>
      <p class="muted">Manage .kraft profiles via CLI: <code>kraftverk profile list|export|inspect|apply|validate</code>.</p>
      <p>Goals: balanced, gaming, compile, workstation, throughput, latency, efficiency, sustained, quiet.</p>
    </div>`;
}

async function renderTelemetry() {
  const data = await api("/api/telemetry");
  const s = data.snapshot || {};
  viewEl.innerHTML = `
    <div class="grid">
      ${metric("CPU %", s.cpu_usage_pct != null ? s.cpu_usage_pct.toFixed(1) : "—")}
      ${metric("Noise", s.noise ? `${s.noise.level} (${s.noise.score.toFixed(2)})` : "—")}
      ${metric("Processes", s.process_count)}
      ${metric("Temp °C", s.temp_c ?? "unavailable")}
      ${metric("Power W", s.power_w ?? "unavailable")}
    </div>
    <p class="muted">Missing sensors are reported as unavailable — never fabricated.</p>
    <ul class="plain">${(s.notes || []).map((n) => `<li>${n}</li>`).join("")}</ul>`;
}

function renderReports() {
  viewEl.innerHTML = `
    <div class="card-free">
      <h2>Reports</h2>
      <p class="muted">Export evidence with <code>kraftverk report --format html</code> or <code>--format json</code>.</p>
    </div>`;
}

async function renderSystem() {
  const data = await api("/api/status");
  const el = data.eligibility || {};
  viewEl.innerHTML = `
    <div class="card-free">
      <h2>System</h2>
      <p>DB: <code>${data.db || "—"}</code></p>
      <p>Fingerprint: <code>${data.fingerprint || "—"}</code></p>
      <p>Hardware policy: <code>${data.hardware_policy || "amd-only-v1"}</code></p>
      <p>Eligibility: <code>${el.compatibility || "—"}</code> · CPU <code>${el.cpu_vendor || "—"}</code></p>
      <p class="muted">${data.agent || ""}</p>
      <p>Active candidate: <code>${data.active_candidate || "none"}</code></p>
    </div>`;
}

function renderSettings() {
  viewEl.innerHTML = `
    <div class="card-free">
      <h2>Settings</h2>
      <p class="muted">Desktop listens on <code>127.0.0.1:47821</code>. Data lives alongside the CLI store. No cloud sync.</p>
      <p class="muted">There is no production hardware bypass flag.</p>
    </div>`;
}

function renderSafety() {
  viewEl.innerHTML = `
    <div class="card-free">
      <h2>Safety Center</h2>
      <ul class="plain">
        <li>AMD-only hardware gate (amd-only-v1) blocks unsupported systems before optimize/benchmark.</li>
        <li>Safe optimizations are reversible and journaled.</li>
        <li>Storage benches write only under Kraftverk temp paths.</li>
        <li>Privileged system changes require a future authenticated agent (also hardware-gated).</li>
        <li><code>kraftverk restore --baseline</code> clears accepted config.</li>
        <li><code>kraftverk doctor</code> / <code>kraftverk compatibility</code> inspect without optimizing.</li>
      </ul>
      <div class="banner">No magic boost buttons. Keep only statistically validated improvements.</div>
    </div>`;
}

preflightHardware()
  .then((ok) => {
    if (ok) return renderOverview();
  })
  .catch((e) => {
    pill.textContent = "offline";
    viewEl.innerHTML = `<p>Failed to load: ${e}</p>`;
  });
