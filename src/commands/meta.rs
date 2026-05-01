//! `meta` subcommand — display metadata key-value pairs in a table.
//!
//! Supports case-insensitive glob filtering via `--filter` and array
//! truncation via `--array-limit`.

use anyhow::Context as _;
use glob::Pattern;

use crate::{
    cli::MetaArgs,
    display::{format_type, format_value, meta_table, term_width},
    gguf::ParsedGguf,
};

/// Run the `meta` subcommand.
pub fn run(args: &MetaArgs) -> anyhow::Result<()> {
    let gguf = ParsedGguf::open(&args.file)
        .with_context(|| format!("failed to open '{}'", args.file.display()))?;

    // Compile optional glob pattern (case-insensitive).
    let pattern: Option<Pattern> = args
        .filter
        .as_deref()
        .map(|f| {
            // Wrap in `*` on both ends if the user didn't include a wildcard,
            // so `llama` matches `llama.context_length` etc.
            Pattern::new(&f.to_lowercase())
        })
        .transpose()
        .with_context(|| "invalid glob pattern")?;

    // Collect and (optionally) filter entries, then sort by key.
    let mut entries: Vec<(&String, &gguf_rs_lib::format::metadata::MetadataValue)> = gguf
        .metadata
        .iter()
        .filter(|(k, _)| {
            if let Some(pat) = &pattern {
                pat.matches(&k.to_lowercase())
            } else {
                true
            }
        })
        .collect();

    entries.sort_by_key(|(k, _)| k.as_str());

    if entries.is_empty() {
        if args.filter.is_some() {
            eprintln!(
                "No metadata keys match the filter {:?}.",
                args.filter.as_deref().unwrap_or("")
            );
        } else {
            eprintln!("This file contains no metadata.");
        }
        return Ok(());
    }

    let rows: Vec<(&str, &str, String)> = entries
        .iter()
        .map(|(k, v)| (k.as_str(), format_type(v), format_value(v, args.array_limit)))
        .collect();

    // tabled needs slices of (&str, &str, &str)
    let row_refs: Vec<(&str, &str, &str)> = rows
        .iter()
        .map(|(k, t, v)| (*k, *t, v.as_str()))
        .collect();

    let width = term_width();
    let table = meta_table(&row_refs, width);
    println!("{}", table);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::MetaArgs;
    use std::path::PathBuf;

    fn missing_args(filter: Option<&str>) -> MetaArgs {
        MetaArgs {
            file: PathBuf::from("/no/such/file.gguf"),
            filter: filter.map(str::to_string),
            array_limit: 8,
        }
    }

    #[test]
    fn meta_on_missing_file_returns_error() {
        assert!(run(&missing_args(None)).is_err());
    }

    #[test]
    fn meta_with_filter_on_missing_file_returns_error() {
        assert!(run(&missing_args(Some("llama.*"))).is_err());
    }
}
