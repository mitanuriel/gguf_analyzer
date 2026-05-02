//! `set` subcommand — write a modified GGUF file with one metadata key changed
//! (or added with `--force`).
//!
//! Writes are always to a new `--output` file; the source is never mutated.
//! Tensor data is streamed byte-for-byte from the source.

use anyhow::Context as _;
use colored::Colorize as _;
use std::fs;
use tracing::{info, instrument};

use gguf_rs_lib::format::metadata::MetadataValue;

use crate::{
    cli::{SetArgs, ValueType},
    display::format_value,
    error::AppError,
    gguf::{ParsedGguf, backup_if_exists, write_modified_gguf},
};

/// Run the `set` subcommand.
#[instrument(skip_all, fields(file = %args.file.display(), key = %args.key))]
pub fn run(args: &SetArgs) -> anyhow::Result<()> {
    let mut gguf = ParsedGguf::open(&args.file)
        .with_context(|| format!("failed to open '{}'", args.file.display()))?;

    // ── Validate key existence ───────────────────────────────────────────────
    let key_exists = gguf.metadata.contains_key(&args.key);
    if !key_exists && !args.force {
        return Err(AppError::KeyNotFound {
            key: args.key.clone(),
            path: args.file.clone(),
        }
        .into());
    }

    // ── Parse the new value ──────────────────────────────────────────────────
    let new_value = parse_value(&args.value, &args.r#type).with_context(|| {
        format!(
            "cannot parse {:?} as type {}",
            args.value,
            args.r#type.type_name()
        )
    })?;

    // ── Dry-run output ───────────────────────────────────────────────────────
    if args.dry_run {
        if key_exists {
            let old = gguf.metadata.get(&args.key).unwrap();
            println!("{} {}", "Would change  :".yellow().bold(), args.key.bold());
            println!("  Old value   : {}", format_value(old, 8).dimmed());
        } else {
            println!("{} {}", "Would create  :".green().bold(), args.key.bold());
        }
        println!("  New value   : {}", format_value(&new_value, 8).cyan());
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
    }

    // ── Apply the change ─────────────────────────────────────────────────────
    gguf.metadata.insert(args.key.clone(), new_value);

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
    info!(key = %args.key, output = %args.output.display(), "set complete");
    Ok(())
}

// ── Value parsing ─────────────────────────────────────────────────────────────

pub fn parse_value(raw: &str, vtype: &ValueType) -> anyhow::Result<MetadataValue> {
    Ok(match vtype {
        ValueType::U8 => MetadataValue::U8(raw.parse::<u8>().context("expected u8")?),
        ValueType::I8 => MetadataValue::I8(raw.parse::<i8>().context("expected i8")?),
        ValueType::U16 => MetadataValue::U16(raw.parse::<u16>().context("expected u16")?),
        ValueType::I16 => MetadataValue::I16(raw.parse::<i16>().context("expected i16")?),
        ValueType::U32 => MetadataValue::U32(raw.parse::<u32>().context("expected u32")?),
        ValueType::I32 => MetadataValue::I32(raw.parse::<i32>().context("expected i32")?),
        ValueType::F32 => MetadataValue::F32(raw.parse::<f32>().context("expected f32")?),
        ValueType::U64 => MetadataValue::U64(raw.parse::<u64>().context("expected u64")?),
        ValueType::I64 => MetadataValue::I64(raw.parse::<i64>().context("expected i64")?),
        ValueType::F64 => MetadataValue::F64(raw.parse::<f64>().context("expected f64")?),
        ValueType::Bool => MetadataValue::Bool(match raw.to_lowercase().as_str() {
            "true" | "1" | "yes" => true,
            "false" | "0" | "no" => false,
            other => anyhow::bail!("expected bool, got {:?}", other),
        }),
        ValueType::String => MetadataValue::String(raw.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_u32() {
        assert_eq!(
            parse_value("42", &ValueType::U32).unwrap(),
            MetadataValue::U32(42)
        );
    }

    #[test]
    fn parse_string() {
        assert_eq!(
            parse_value("hello", &ValueType::String).unwrap(),
            MetadataValue::String("hello".to_string())
        );
    }

    #[test]
    fn parse_bool_variants() {
        for s in &["true", "1", "yes"] {
            assert_eq!(
                parse_value(s, &ValueType::Bool).unwrap(),
                MetadataValue::Bool(true)
            );
        }
        for s in &["false", "0", "no"] {
            assert_eq!(
                parse_value(s, &ValueType::Bool).unwrap(),
                MetadataValue::Bool(false)
            );
        }
    }

    #[test]
    fn parse_bad_u32_errors() {
        assert!(parse_value("not_a_number", &ValueType::U32).is_err());
    }

    #[test]
    fn set_on_missing_file_errors() {
        use crate::cli::SetArgs;
        use std::path::PathBuf;
        let args = SetArgs {
            file: PathBuf::from("/no/such/file.gguf"),
            key: "general.name".to_string(),
            value: "test".to_string(),
            r#type: ValueType::String,
            output: PathBuf::from("/tmp/out.gguf"),
            force: false,
            backup: false,
            dry_run: false,
        };
        assert!(run(&args).is_err());
    }
}
