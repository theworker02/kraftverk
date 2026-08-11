use anyhow::{anyhow, Result};

use crate::output::OutputOpts;
#[cfg(feature = "dev-simulate")]
use crate::output::{print_json, println_human};

#[cfg(feature = "dev-simulate")]
pub fn run(out: &OutputOpts, profile: &str) -> Result<()> {
    use kraftverk_system::mock::{MockConfig, MockPlatform};
    use kraftverk_system::Platform;

    let mut cfg = MockConfig::default();
    match profile {
        "quiet" => cfg.noise_stddev = 0.005,
        "noisy" => cfg.noise_stddev = 0.08,
        "fast" => cfg.throttle_multiplier = 1.0,
        "slow" => {
            cfg.throttle_after_applies = Some(0);
            cfg.throttle_multiplier = 0.85;
        }
        other => {
            return Err(anyhow!(
                "unknown simulation profile '{other}' (quiet|noisy|fast|slow)"
            ))
        }
    }
    let platform = MockPlatform::new(cfg);
    let caps = platform.capabilities();
    if out.json {
        print_json(&serde_json::json!({
            "ok": true,
            "simulated": true,
            "profile": profile,
            "capabilities": caps,
            "score_multiplier": platform.score_multiplier(),
            "note": "Dev-only simulation; never used as real telemetry."
        }));
    } else {
        println_human(out, format!("Simulated machine profile: {profile}"));
        println_human(
            out,
            "This is a development aid (feature dev-simulate). Not production evidence.",
        );
        println_human(
            out,
            format!("Mock score multiplier: {:.3}", platform.score_multiplier()),
        );
    }
    Ok(())
}

#[cfg(not(feature = "dev-simulate"))]
pub fn run(_out: &OutputOpts, _profile: &str) -> Result<()> {
    Err(anyhow!(
        "simulate-machine is available only in dev builds \
         (`cargo run -p kraftverk-cli --features dev-simulate -- dev simulate-machine <profile>`)"
    ))
}
