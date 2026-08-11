//! Kraftverk CLI.

mod commands;
mod engine;
mod output;

use clap::{Parser, Subcommand, ValueEnum};
use tracing_subscriber::EnvFilter;

use commands::gate::HardwareGateError;

const VERSION_INFO: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("CARGO_PKG_NAME"), ")");

#[derive(Debug, Parser)]
#[command(
    name = "kraftverk",
    version = VERSION_INFO,
    about = "Evidence-driven systems performance platform (AMD-exclusive)",
    long_about = "Kraftverk measures real workloads, experiments with reversible settings, \
and only keeps changes that survive statistical validation. It is not a PC cleaner.\n\n\
Hardware policy amd-only-v1: x86/x86_64 + AMD CPU; NVIDIA/Intel GPU or Intel CPU blocked.\n\
Independent product — not affiliated with or endorsed by AMD."
)]
struct Cli {
    /// Emit machine-readable JSON.
    #[arg(long, global = true)]
    json: bool,

    /// Suppress non-essential output.
    #[arg(long, short = 'q', global = true)]
    quiet: bool,

    /// Verbose logging.
    #[arg(long, short = 'v', global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Discover hardware / OS facts and machine fingerprint.
    Inspect,
    /// Inspect-only AMD hardware compatibility report (no gate).
    Compatibility,
    /// Inspect-only CPU/GPU inventory via CPUID + PCI IDs (no gate).
    Hardware,
    /// AMD CPU / GPU capability surfaces.
    Amd {
        #[command(subcommand)]
        target: AmdCmd,
    },
    /// Create a baseline Kraft Index (normalized to 10,000).
    Baseline {
        #[arg(long, default_value_t = 2)]
        warmup: usize,
        #[arg(long, default_value_t = 5)]
        samples: usize,
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
    /// Run KraftBench without creating a baseline.
    Benchmark {
        #[arg(long, default_value_t = 1)]
        warmup: usize,
        #[arg(long, default_value_t = 3)]
        samples: usize,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// Sustained window, e.g. 10m / 30s (0 or omit = off).
        #[arg(long)]
        sustained: Option<String>,
    },
    /// Search reversible candidates and validate winners.
    Optimize {
        #[arg(long, value_enum, default_value_t = ModeArg::Safe)]
        mode: ModeArg,
        /// Goal bias: balanced, gaming, compile, workstation, throughput, latency, efficiency, sustained, quiet.
        #[arg(long, default_value = "balanced")]
        goal: String,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[arg(long, default_value_t = 12)]
        max_experiments: usize,
        #[arg(long, default_value_t = 600)]
        time_budget_secs: u64,
        #[arg(long)]
        max_temp: Option<f64>,
        #[arg(long)]
        max_power: Option<f64>,
        #[arg(long)]
        max_workers: Option<usize>,
        /// Resume a previous optimize session by id (prefix ok).
        #[arg(long)]
        resume: Option<String>,
    },
    /// Show current status, baseline, and active candidate.
    Status,
    /// List recent experiments.
    History {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Explain an experiment by id (prefix ok).
    Explain { experiment: String },
    /// Compare two experiments.
    Compare { a: String, b: String },
    /// List optimization profiles and support status.
    Profiles,
    /// Profile export / inspect / apply / validate / recommend.
    Profile {
        #[command(subcommand)]
        action: ProfileCmd,
    },
    /// Roll back active accepted changes.
    Restore {
        /// Also note that measurement baseline is retained (identity restore).
        #[arg(long)]
        baseline: bool,
    },
    /// Show platform capabilities.
    Capabilities,
    /// Health / environment checks.
    Doctor,
    /// Derived insights from experiment history.
    Insights,
    /// Show experiment lineage.
    Lineage { experiment: String },
    /// List optimize sessions.
    Sessions {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Export HTML/JSON evidence report.
    Report {
        #[arg(long)]
        experiment: Option<String>,
        #[arg(long, default_value = "html")]
        format: String,
        #[arg(long, short = 'o')]
        output: Option<String>,
    },
    /// Chase (time) an external command.
    Chase {
        #[arg(long, default_value_t = 1)]
        samples: usize,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Analyze an experiment id or `recent`.
    Analyze {
        #[arg(default_value = "recent")]
        target: String,
    },
    /// Export or verify an evidence receipt.
    Receipt {
        /// Experiment id (prefix ok). Ignored when `--verify` is set.
        experiment: Option<String>,
        #[arg(long, short = 'o')]
        output: Option<String>,
        /// Verify an existing `.kraft-receipt.json` file.
        #[arg(long)]
        verify: Option<String>,
    },
    /// Development helpers (feature-gated).
    Dev {
        #[command(subcommand)]
        action: DevCmd,
    },
}

#[derive(Debug, Subcommand)]
enum AmdCmd {
    /// Ryzen / AMD CPU topology hints (honest — no fabricated CCD maps).
    Cpu,
    /// AMD GPU enumeration (PCI 0x1002).
    Gpu,
}

#[derive(Debug, Subcommand)]
enum ProfileCmd {
    List,
    Recommend { goal: String },
    Export { path: String },
    Inspect { path: String },
    Validate { path: String },
    Apply { path: String },
}

#[derive(Debug, Subcommand)]
enum DevCmd {
    /// Simulate a mock machine (requires --features dev-simulate).
    SimulateMachine {
        #[arg(default_value = "quiet")]
        profile: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ModeArg {
    Safe,
    Balanced,
    Aggressive,
}

impl From<ModeArg> for kraftverk_core::OptimizeMode {
    fn from(m: ModeArg) -> Self {
        match m {
            ModeArg::Safe => Self::Safe,
            ModeArg::Balanced => Self::Balanced,
            ModeArg::Aggressive => Self::Aggressive,
        }
    }
}

fn command_skips_hardware_gate(cmd: &Commands) -> bool {
    matches!(
        cmd,
        Commands::Compatibility | Commands::Hardware | Commands::Amd { .. }
    )
}

fn main() {
    let cli = Cli::parse();
    init_tracing(cli.verbose, cli.quiet);

    let out = output::OutputOpts {
        json: cli.json,
        quiet: cli.quiet,
        verbose: cli.verbose,
    };

    // Shared pre-dispatch AMD eligibility gate (except inspect-only surfaces).
    if !command_skips_hardware_gate(&cli.command) {
        if let Err(gate) = commands::gate::require_eligible() {
            emit_hardware_error(&out, &gate);
            std::process::exit(gate.exit_code);
        }
    }

    let result = match cli.command {
        Commands::Inspect => commands::inspect::run(&out),
        Commands::Compatibility => commands::compatibility::run(&out),
        Commands::Hardware => commands::hardware::run(&out),
        Commands::Amd { target } => {
            let t = match target {
                AmdCmd::Cpu => commands::amd::AmdTarget::Cpu,
                AmdCmd::Gpu => commands::amd::AmdTarget::Gpu,
            };
            commands::amd::run(&out, t)
        }
        Commands::Baseline {
            warmup,
            samples,
            seed,
        } => commands::baseline::run(&out, warmup, samples, seed),
        Commands::Benchmark {
            warmup,
            samples,
            seed,
            sustained,
        } => commands::benchmark::run(&out, warmup, samples, seed, sustained.as_deref()),
        Commands::Optimize {
            mode,
            goal,
            seed,
            max_experiments,
            time_budget_secs,
            max_temp,
            max_power,
            max_workers,
            resume,
        } => commands::optimize::run(
            &out,
            mode.into(),
            &goal,
            seed,
            max_experiments,
            time_budget_secs,
            max_temp,
            max_power,
            max_workers,
            resume.as_deref(),
        ),
        Commands::Status => commands::status::run(&out),
        Commands::History { limit } => commands::history::run(&out, limit),
        Commands::Explain { experiment } => commands::explain::run(&out, &experiment),
        Commands::Compare { a, b } => commands::compare::run(&out, &a, &b),
        Commands::Profiles => commands::profiles::run(&out),
        Commands::Profile { action } => match action {
            ProfileCmd::List => commands::profile_cmd::list(&out),
            ProfileCmd::Recommend { goal } => commands::profile_cmd::recommend(&out, &goal),
            ProfileCmd::Export { path } => commands::profile_cmd::export(&out, &path),
            ProfileCmd::Inspect { path } => commands::profile_cmd::inspect(&out, &path),
            ProfileCmd::Validate { path } => commands::profile_cmd::validate(&out, &path),
            ProfileCmd::Apply { path } => commands::profile_cmd::apply(&out, &path),
        },
        Commands::Restore { baseline } => commands::restore::run(&out, baseline),
        Commands::Capabilities => commands::capabilities::run(&out),
        Commands::Doctor => commands::doctor::run(&out),
        Commands::Insights => commands::insights::run(&out),
        Commands::Lineage { experiment } => commands::lineage::run(&out, &experiment),
        Commands::Sessions { limit } => commands::sessions::run(&out, limit),
        Commands::Report {
            experiment,
            format,
            output,
        } => commands::report::run(&out, experiment.as_deref(), &format, output.as_deref()),
        Commands::Chase { samples, command } => commands::chase::run(&out, &command, samples),
        Commands::Analyze { target } => commands::analyze::run(&out, &target),
        Commands::Receipt {
            experiment,
            output,
            verify,
        } => {
            if verify.is_none() && experiment.is_none() {
                Err(anyhow::anyhow!(
                    "receipt requires an experiment id or --verify <path>"
                ))
            } else {
                commands::receipt::run(
                    &out,
                    experiment.as_deref().unwrap_or(""),
                    output.as_deref(),
                    verify.as_deref(),
                )
            }
        }
        Commands::Dev { action } => match action {
            DevCmd::SimulateMachine { profile } => commands::dev::run(&out, &profile),
        },
    };

    if let Err(e) = result {
        if let Some(gate) = e.downcast_ref::<HardwareGateError>() {
            emit_hardware_error(&out, gate);
            std::process::exit(gate.exit_code);
        }
        if out.json {
            eprintln!(
                "{}",
                serde_json::json!({
                    "ok": false,
                    "error": e.to_string(),
                })
            );
        } else if !out.quiet {
            eprintln!("error: {e}");
        }
        std::process::exit(1);
    }
}

fn emit_hardware_error(out: &output::OutputOpts, gate: &HardwareGateError) {
    if out.json {
        eprintln!(
            "{}",
            serde_json::json!({
                "ok": false,
                "error": gate.message,
                "exit_code": gate.exit_code,
                "policy": gate.eligibility.policy,
                "compatibility": gate.eligibility.compatibility.as_str(),
                "rejection_reasons": gate.eligibility.rejection_reasons.iter().map(|r| r.message()).collect::<Vec<_>>(),
            })
        );
    } else if !out.quiet {
        eprintln!("error: {}", gate.message);
        eprintln!(
            "hint: run `kraftverk compatibility` or `kraftverk hardware` for an inspect-only report"
        );
    }
}

fn init_tracing(verbose: bool, quiet: bool) {
    let level = if quiet {
        "error"
    } else if verbose {
        "debug"
    } else {
        "warn"
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}
