//! `tensors` subcommand — display tensor inventory in a table.

use anyhow::Context as _;
use glob::Pattern;
use tracing::{debug, instrument};

use crate::{
    cli::TensorsArgs,
    display::{tensor_table, term_width},
    gguf::ParsedGguf,
};

/// Run the `tensors` subcommand.
#[instrument(skip_all, fields(file = %args.file.display(), filter = ?args.filter))]
pub fn run(args: &TensorsArgs) -> anyhow::Result<()> {
    let gguf = ParsedGguf::open(&args.file)
        .with_context(|| format!("failed to open '{}'", args.file.display()))?;

    // Compile optional glob pattern (case-insensitive).
    let pattern: Option<Pattern> = args
        .filter
        .as_deref()
        .map(|f| Pattern::new(&f.to_lowercase()))
        .transpose()
        .context("invalid glob pattern")?;

    let infos: Vec<_> = gguf
        .tensor_infos
        .iter()
        .filter(|ti| {
            if let Some(pat) = &pattern {
                pat.matches(&ti.name.to_lowercase())
            } else {
                true
            }
        })
        .cloned()
        .collect();

    if infos.is_empty() {
        if args.filter.is_some() {
            eprintln!(
                "No tensors match the filter {:?}.",
                args.filter.as_deref().unwrap_or("")
            );
        } else {
            eprintln!("This file contains no tensors.");
        }
        return Ok(());
    }

    let width = term_width();
    let table = tensor_table(&infos, width);
    debug!(tensors = infos.len(), "rendering tensor table");
    println!("{}", table);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::TensorsArgs;
    use std::path::PathBuf;

    #[test]
    fn tensors_on_missing_file_returns_error() {
        let args = TensorsArgs {
            file: PathBuf::from("/no/such/file.gguf"),
            filter: None,
        };
        assert!(run(&args).is_err());
    }
}
