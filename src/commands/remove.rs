//! `remove` subcommand — write a modified GGUF file with one metadata key deleted.
//!
//! Writes are always to a new `--output` file; the source is never mutated.
//! Tensor data is streamed byte-for-byte from the source.

use anyhow::Context as _;
use std::fs;

use crate::{
    cli::RemoveArgs,
    display::format_value,
    error::AppError,
    gguf::{write_modified_gguf, ParsedGguf},
};

/// Run the `remove` subcommand.
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
        println!("Would remove  : {}", args.key);
        println!("  Value       : {}", format_value(old, 8));
        println!("  Output file : {}", args.output.display());
        println!("(dry-run — no files written)");
        return Ok(());
    }

    // ── Guard output file ────────────────────────────────────────────────────
    if args.output.exists() && !args.force {
        return Err(AppError::OutputExists { path: args.output.clone() }.into());
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
        "Written: '{}' ({} bytes)",
        args.output.display(),
        fs::metadata(&args.output)
            .map(|m| m.len().to_string())
            .unwrap_or_else(|_| "?".to_string())
    );
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
            file:    PathBuf::from("/no/such/file.gguf"),
            key:     "general.name".to_string(),
            output:  PathBuf::from("/tmp/out.gguf"),
            force:   false,
            dry_run: false,
        };
        assert!(run(&args).is_err());
    }
}
