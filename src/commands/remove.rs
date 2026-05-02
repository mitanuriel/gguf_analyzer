//! `remove` subcommand — write a modified GGUF file with one metadata key deleted.
//!
//! Writes are always to a new `--output` file; the source is never mutated.
//! Tensor data is streamed byte-for-byte from the source.

use anyhow::Context as _;
use colored::Colorize as _;
use std::fs;
use tracing::{info, instrument};

use crate::{
    cli::RemoveArgs,
    display::format_value,
    error::AppError,
    gguf::{ParsedGguf, backup_if_exists, write_modified_gguf},
};

/// Run the `remove` subcommand.
#[instrument(skip_all, fields(file = %args.file.display(), key = %args.key))]
pub fn run(args: &RemoveArgs) -> anyhow::Result<()> {
    let mut gguf = ParsedGguf::open(&args.file)
        .with_context(|| format!("failed to open '{}'", args.file.display()))?;

    // ── Validate key existence ───────────────────────────────────────────────
    if !gguf.metadata.contains_key(&args.key) {
        return Err(AppError::KeyNotFound {
            key: args.key.clone(),
            path: args.file.clone(),
        }
        .into());
    }

    // ── Dry-run output ───────────────────────────────────────────────────────
    if args.dry_run {
        let old = gguf.metadata.get(&args.key).unwrap();
        println!("{} {}", "Would remove  :".yellow().bold(), args.key.bold());
        println!("  Value       : {}", format_value(old, 8).dimmed());
        println!("  Output file : {}", args.output.display());
        println!("{}", "(dry-run — no files written)".dimmed());
        return Ok(());
    }

    // ── Guard output file ────────────────────────────────────────────────────
    if args.output.exists() && !args.force {
        return Err(AppError::OutputExists {
            path: args.output.clone(),
        }
        .into());
    }

    // ── Optional backup of existing output ───────────────────────────────────
    if args.backup
        && let Some(bak) = backup_if_exists(&args.output)?
    {
        eprintln!("{} '{}'", "Backup :".blue().bold(), bak.display());
        // In-place edit: source path == output path. We just renamed it to
        // `.bak`, so make the writer read tensor bytes from there.
        if gguf.path == args.output {
            gguf.path = bak;
        }
    }

    // ── Apply the change ─────────────────────────────────────────────────────
    gguf.metadata.remove(&args.key);

    // ── Write ────────────────────────────────────────────────────────────────
    write_modified_gguf(
        &gguf.path,
        gguf.tensor_data_offset,
        &gguf.metadata,
        &gguf.tensor_infos,
        gguf.alignment,
        &args.output,
    )
    .with_context(|| format!("write output '{}'", args.output.display()))?;

    eprintln!(
        "{} '{}' ({} bytes)",
        "Written:".green().bold(),
        args.output.display(),
        fs::metadata(&args.output)
            .map(|m| m.len().to_string())
            .unwrap_or_else(|_| "?".to_string())
    );
    info!(key = %args.key, output = %args.output.display(), "remove complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::RemoveArgs;
    use std::path::PathBuf;

    #[test]
    fn remove_on_missing_file_errors() {
        let args = RemoveArgs {
            file: PathBuf::from("/no/such/file.gguf"),
            key: "general.name".to_string(),
            output: PathBuf::from("/tmp/out.gguf"),
            force: false,
            backup: false,
            dry_run: false,
        };
        assert!(run(&args).is_err());
    }
}
