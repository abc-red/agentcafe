use agentcafe_core::{ScanOptions, generate_report};
use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "agentcafe")]
#[command(about = "Agent Cafe local diagnostics")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        schema: Option<PathBuf>,
        #[arg(long, value_name = "PATH")]
        home: Option<PathBuf>,
        #[arg(long = "workspace-root", value_name = "PATH")]
        workspace_root: Option<PathBuf>,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Doctor {
            json,
            schema,
            home,
            workspace_root,
        } => {
            if !json {
                bail!("code=invalid_params: doctor currently requires --json");
            }
            let mut options = ScanOptions::from_env();
            if let Some(home) = home {
                options.home = home;
            }
            if let Some(workspace_root) = workspace_root {
                options.workspace_root = workspace_root;
            }
            let report = generate_report(&options);
            let value = serde_json::to_value(&report).context("code=serialize_failed")?;
            if let Some(schema_path) = schema {
                validate_schema(&value, &schema_path, &report.trace_id)?;
            }
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
    }
    Ok(())
}

fn validate_schema(value: &serde_json::Value, schema_path: &PathBuf, trace_id: &str) -> Result<()> {
    let schema_text = fs::read_to_string(schema_path).context("code=schema_read_failed")?;
    let schema: serde_json::Value =
        serde_json::from_str(&schema_text).context("code=schema_parse_failed")?;
    let validator = jsonschema::validator_for(&schema).context("code=schema_compile_failed")?;
    if let Err(err) = validator.validate(value) {
        bail!(
            "code=diagnostic_schema_invalid trace_id={} detail={}",
            trace_id,
            err.instance_path
        );
    }
    Ok(())
}
