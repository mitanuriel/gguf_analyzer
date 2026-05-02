use gguf_analyzer::{cli, commands};

use clap::CommandFactory;
use clap_complete::generate;
use colored::Colorize as _;
use std::io;
use tracing_subscriber::{EnvFilter, fmt};

use cli::{Cli, Command};

fn main() {
    // Initialise tracing.  Set RUST_LOG=debug (or trace/info/warn) to see spans.
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .with_target(false)
        .compact()
        .init();

    if let Err(e) = run() {
        eprintln!("{} {:#}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    use clap::Parser;
    let cli = Cli::parse();

    match &cli.command {
        Command::Info(args) => commands::info::run(args),
        Command::Meta(args) => commands::meta::run(args),
        Command::Tensors(args) => commands::tensors::run(args),
        Command::Set(args) => commands::set::run(args),
        Command::Remove(args) => commands::remove::run(args),
        Command::Export(args) => commands::export::run(args),
        Command::Fetch(args) => commands::fetch::run(args),
        Command::ModelCard(args) => commands::model_card::run(args),
        Command::ApplySampling(args) => commands::apply_sampling::run(args),
        Command::Completions(args) => {
            let mut cmd = Cli::command();
            generate(args.shell, &mut cmd, "gguf-analyzer", &mut io::stdout());
            Ok(())
        }
    }
}
