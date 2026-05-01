mod cli;
mod commands;
pub mod display;
mod error;
pub mod gguf;

use anyhow::Context as _;
use clap::CommandFactory;
use clap_complete::generate;
use std::io;

use cli::{Cli, Command};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    use clap::Parser;
    let cli = Cli::parse();

    match &cli.command {
        Command::Info(args)        => commands::info::run(args),
        Command::Meta(args)        => commands::meta::run(args),
        Command::Tensors(args)     => commands::tensors::run(args),
        Command::Set(args)         => commands::set::run(args),
        Command::Remove(args)      => commands::remove::run(args),
        Command::Export(args)      => commands::export::run(args),
        Command::Completions(args) => {
            let mut cmd = Cli::command();
            generate(args.shell, &mut cmd, "gguf-analyzer", &mut io::stdout());
            Ok(())
        }
    }
}
