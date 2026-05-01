//! `export` subcommand — export metadata to JSON, Markdown, or CSV.

use std::{
    fs,
    io::{self, Write},
};

use anyhow::Context as _;
use serde_json::{json, Map, Value as JsonValue};
use tracing::{debug, instrument};

use crate::{
    cli::{ExportArgs, ExportFormat},
    display::{format_type, format_value},
    error::AppError,
    gguf::ParsedGguf,
};

/// Run the `export` subcommand.
#[instrument(skip_all, fields(file = %args.file.display(), format = ?args.format))]
pub fn run(args: &ExportArgs) -> anyhow::Result<()> {
    let gguf = ParsedGguf::open(&args.file)
        .with_context(|| format!("failed to open '{}'", args.file.display()))?;

    // Sort entries by key for deterministic output.
    let mut entries: Vec<_> = gguf.metadata.iter().collect();
    entries.sort_by_key(|(k, _)| k.as_str());
    debug!(entries = entries.len(), "exporting metadata");

    let output = match args.format {
        ExportFormat::Json     => export_json(&entries, args.array_limit)?,
        ExportFormat::Markdown => export_markdown(&entries, args.array_limit),
        ExportFormat::Csv      => export_csv(&entries, args.array_limit),
    };

    match &args.output {
        None => {
            io::stdout()
                .write_all(output.as_bytes())
                .context("write to stdout")?;
        }
        Some(dest) => {
            fs::write(dest, &output)
                .map_err(|e| AppError::io(dest, e))
                .with_context(|| format!("write export to '{}'", dest.display()))?;
            eprintln!("Exported to '{}'.", dest.display());
        }
    }
    Ok(())
}

// ── JSON ──────────────────────────────────────────────────────────────────────

fn export_json(
    entries: &[(&String, &gguf_rs_lib::format::metadata::MetadataValue)],
    array_limit: usize,
) -> anyhow::Result<String> {
    let mut map = Map::new();
    for (k, v) in entries {
        map.insert(
            k.to_string(),
            json!({
                "type":  format_type(v),
                "value": format_value(v, array_limit),
            }),
        );
    }
    let out = serde_json::to_string_pretty(&JsonValue::Object(map))
        .context("serialise metadata to JSON")?;
    Ok(out + "\n")
}

// ── Markdown ──────────────────────────────────────────────────────────────────

fn export_markdown(
    entries: &[(&String, &gguf_rs_lib::format::metadata::MetadataValue)],
    array_limit: usize,
) -> String {
    let mut out = String::new();
    out.push_str("| Key | Type | Value |\n");
    out.push_str("|-----|------|-------|\n");
    for (k, v) in entries {
        let value = format_value(v, array_limit).replace('|', "\\|");
        out.push_str(&format!("| `{}` | `{}` | {} |\n", k, format_type(v), value));
    }
    out
}

// ── CSV ───────────────────────────────────────────────────────────────────────

fn export_csv(
    entries: &[(&String, &gguf_rs_lib::format::metadata::MetadataValue)],
    array_limit: usize,
) -> String {
    let mut out = String::from("key,type,value\n");
    for (k, v) in entries {
        let value = format_value(v, array_limit).replace('"', "\"\"");
        out.push_str(&format!("{},{},\"{}\"\n", k, format_type(v), value));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::ExportArgs;
    use std::path::PathBuf;

    fn args(format: ExportFormat) -> ExportArgs {
        ExportArgs {
            file: PathBuf::from("/no/such/file.gguf"),
            output: None,
            format,
            array_limit: 8,
        }
    }

    #[test]
    fn export_json_on_missing_file_errors() {
        assert!(run(&args(ExportFormat::Json)).is_err());
    }

    #[test]
    fn export_markdown_on_missing_file_errors() {
        assert!(run(&args(ExportFormat::Markdown)).is_err());
    }

    #[test]
    fn export_csv_on_missing_file_errors() {
        assert!(run(&args(ExportFormat::Csv)).is_err());
    }

    #[test]
    fn markdown_escapes_pipe() {
        use gguf_rs_lib::format::metadata::MetadataValue;
        let key = "test.key".to_string();
        let val = MetadataValue::String("a|b".to_string());
        let entries = vec![(&key, &val)];
        let md = export_markdown(&entries, 8);
        assert!(md.contains("a\\|b"));
    }

    #[test]
    fn csv_header_row() {
        let csv = export_csv(&[], 8);
        assert!(csv.starts_with("key,type,value\n"));
    }
}
