//! HTML/JSON experiment reports (evidence-based; no fabricated metrics).

use kraftverk_core::Experiment;
use serde_json::json;

pub fn report_json(experiments: &[Experiment], title: &str) -> serde_json::Value {
    json!({
        "format": "kraftverk.report",
        "version": 1,
        "title": title,
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "experiments": experiments.iter().map(|e| json!({
            "id": e.id.to_string(),
            "kind": e.kind,
            "decision": e.decision,
            "decision_reason": e.decision_reason,
            "stability": e.stability.as_str(),
            "comparison_class": e.comparison_class.map(|c| c.as_str().to_string()),
            "index_mean": e.index_summary.as_ref().map(|s| s.mean),
            "index_cov": e.index_summary.as_ref().map(|s| s.cov),
            "kraft_index": e.kraft_index.as_ref().map(|k| k.score),
            "candidate": e.candidate,
            "created_at": e.created_at.to_rfc3339(),
            "sample_count": e.index_samples.len(),
        })).collect::<Vec<_>>(),
    })
}

pub fn report_html(experiments: &[Experiment], title: &str) -> String {
    let mut rows = String::new();
    for e in experiments {
        let score = e
            .index_summary
            .as_ref()
            .map(|s| format!("{:.1}", s.mean))
            .unwrap_or_else(|| "—".into());
        let class = e
            .comparison_class
            .map(|c| c.as_str().to_string())
            .unwrap_or_else(|| "—".into());
        rows.push_str(&format!(
            "<tr><td><code>{}</code></td><td>{:?}</td><td>{:?}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            e.id,
            e.kind,
            e.decision,
            e.stability.as_str(),
            score,
            class,
            html_escape(&e.candidate.summary_line()),
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<title>{title}</title>
<style>
  :root {{ --bg:#0f1419; --fg:#e7ecf1; --muted:#8b9aab; --accent:#3d9a8b; --line:#243040; }}
  body {{ font-family: "IBM Plex Sans", "Segoe UI", sans-serif; background:var(--bg); color:var(--fg); margin:0; padding:2rem; }}
  h1 {{ font-weight:550; letter-spacing:-0.02em; }}
  p.sub {{ color:var(--muted); max-width:40rem; }}
  table {{ border-collapse:collapse; width:100%; margin-top:1.5rem; font-size:0.92rem; }}
  th, td {{ border-bottom:1px solid var(--line); padding:0.65rem 0.5rem; text-align:left; vertical-align:top; }}
  th {{ color:var(--muted); font-weight:500; }}
  code {{ color:var(--accent); }}
  footer {{ margin-top:2rem; color:var(--muted); font-size:0.85rem; }}
</style>
</head>
<body>
  <h1>{title}</h1>
  <p class="sub">Evidence report from measured Kraftverk experiments. Scores are from real benchmarks — nothing here is invented.</p>
  <table>
    <thead><tr><th>ID</th><th>Kind</th><th>Decision</th><th>Stability</th><th>Index</th><th>Class</th><th>Candidate</th></tr></thead>
    <tbody>
{rows}
    </tbody>
  </table>
  <footer>Generated {ts} · Kraftverk report format v1</footer>
</body>
</html>
"#,
        title = html_escape(title),
        rows = rows,
        ts = chrono::Utc::now().to_rfc3339(),
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
