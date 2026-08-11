use anyhow::Result;
use kraftverk_agent::{
    agent_connected, default_endpoint, run_agent_server, trust_boundary_summary, AgentClient,
};
use uuid::Uuid;

use crate::output::{print_json, println_human, OutputOpts};

#[derive(Debug, Clone, Copy)]
pub enum AgentAction {
    Serve,
    Status,
}

pub fn run(out: &OutputOpts, action: AgentAction) -> Result<()> {
    match action {
        AgentAction::Serve => {
            println_human(
                out,
                format!(
                    "Starting kraftverk-agent on {} (local IPC only)",
                    default_endpoint()
                ),
            );
            println_human(
                out,
                "If power-scheme changes fail, re-run this command from an elevated terminal.",
            );
            println_human(out, trust_boundary_summary());
            run_agent_server().map_err(|e| anyhow::anyhow!(e))?;
            Ok(())
        }
        AgentAction::Status => {
            let connected = agent_connected();
            if out.json {
                let mut body = serde_json::json!({
                    "ok": true,
                    "connected": connected,
                    "endpoint": default_endpoint(),
                    "trust_boundary": trust_boundary_summary(),
                });
                if connected {
                    if let Ok(mut c) = AgentClient::connect_default() {
                        if let Ok(resp) =
                            c.request(kraftverk_agent::AgentRequest::Health { id: Uuid::new_v4() })
                        {
                            body["health"] = serde_json::to_value(resp).unwrap_or_default();
                        }
                    }
                }
                print_json(&body);
            } else {
                println_human(out, "Kraftverk agent status");
                println_human(
                    out,
                    format!(
                        "  [{}] connected — {}",
                        if connected { "ok" } else { "!!" },
                        default_endpoint()
                    ),
                );
                if !connected {
                    println_human(
                        out,
                        "  Start with: kraftverk agent serve  (elevate if power schemes require it)",
                    );
                }
                println_human(out, trust_boundary_summary());
            }
            Ok(())
        }
    }
}
