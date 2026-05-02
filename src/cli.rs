//! CLI argument definitions.
//!
//! Every subcommand is a plain Rust struct derived with [`clap::Parser`].
//! The top-level entry point is [`Cli`], which owns a [`Command`] enum that
//! dispatches to the right subcommand handler.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

// ── Top-level ─────────────────────────────────────────────────────────────────

/// A CLI tool to explore and edit the metadata of GGUF model files.
#[derive(Debug, Parser)]
#[command(
    name = "gguf-analyzer",
    version,
    about,
    long_about = None,
    propagate_version = true,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

// ── Subcommands ───────────────────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print a high-level summary of the GGUF file (version, counts, size).
    Info(InfoArgs),

    /// List metadata key-value pairs.
    Meta(MetaArgs),

    /// List tensor info (name, shape, type, offset, size).
    Tensors(TensorsArgs),

    /// Set or update a metadata key, writing to a new output file.
    Set(SetArgs),

    /// Remove a metadata key, writing to a new output file.
    Remove(RemoveArgs),

    /// Export metadata to a file (JSON / Markdown / CSV).
    Export(ExportArgs),

    /// Print shell completion script to stdout.
    Completions(CompletionsArgs),
}

// ── info ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
pub struct InfoArgs {
    /// Path to the GGUF file to inspect.
    pub file: PathBuf,
}

// ── meta ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
pub struct MetaArgs {
    /// Path to the GGUF file to inspect.
    pub file: PathBuf,

    /// Filter keys using a glob pattern (case-insensitive).
    ///
    /// Examples: `general.*`, `llama.*`, `tokenizer.ggml.*`
    #[arg(short, long, value_name = "PATTERN")]
    pub filter: Option<String>,

    /// Maximum number of array elements to show per value (0 = unlimited).
    #[arg(long, default_value = "8", value_name = "N")]
    pub array_limit: usize,
}

// ── tensors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
pub struct TensorsArgs {
    /// Path to the GGUF file to inspect.
    pub file: PathBuf,

    /// Filter tensor names using a glob pattern (case-insensitive).
    #[arg(short, long, value_name = "PATTERN")]
    pub filter: Option<String>,
}

// ── set ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
pub struct SetArgs {
    /// Path to the source GGUF file.
    pub file: PathBuf,

    /// Metadata key to set (e.g. `general.name`).
    #[arg(long)]
    pub key: String,

    /// New value (parsed according to --type).
    #[arg(long)]
    pub value: String,

    /// Value type to use when serialising the new value.
    #[arg(short = 't', long, value_name = "TYPE")]
    pub r#type: ValueType,

    /// Destination path for the new GGUF file.
    #[arg(short, long, value_name = "FILE")]
    pub output: PathBuf,

    /// Overwrite the output file if it already exists.
    #[arg(long)]
    pub force: bool,

    /// Before overwriting an existing output file, rename it to `<output>.bak`.
    ///
    /// Useful when chaining edits onto the same target file: the previous
    /// version is preserved as a sibling `.bak`, so you can roll back with a
    /// simple `mv` if something goes wrong. Has no effect if the output file
    /// does not yet exist, or in `--dry-run` mode.
    #[arg(long)]
    pub backup: bool,

    /// Show what would change without writing any bytes to disk.
    #[arg(long)]
    pub dry_run: bool,
}

// ── remove ────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
pub struct RemoveArgs {
    /// Path to the source GGUF file.
    pub file: PathBuf,

    /// Metadata key to remove.
    #[arg(long)]
    pub key: String,

    /// Destination path for the new GGUF file.
    #[arg(short, long, value_name = "FILE")]
    pub output: PathBuf,

    /// Overwrite the output file if it already exists.
    #[arg(long)]
    pub force: bool,

    /// Before overwriting an existing output file, rename it to `<output>.bak`.
    #[arg(long)]
    pub backup: bool,

    /// Show what would change without writing any bytes to disk.
    #[arg(long)]
    pub dry_run: bool,
}

// ── export ────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
pub struct ExportArgs {
    /// Path to the GGUF file to export.
    pub file: PathBuf,

    /// Output file path (stdout if omitted).
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Export format.
    #[arg(short = 'f', long, default_value = "json", value_name = "FORMAT")]
    pub format: ExportFormat,

    /// Maximum number of array elements to show per value (0 = unlimited).
    #[arg(long, default_value = "8", value_name = "N")]
    pub array_limit: usize,
}

/// Supported export formats.
#[derive(Debug, Clone, ValueEnum)]
pub enum ExportFormat {
    Json,
    Markdown,
    Csv,
}

// ── completions ───────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
pub struct CompletionsArgs {
    /// Shell to generate completions for.
    pub shell: Shell,
}

// ── Value type enum ───────────────────────────────────────────────────────────

/// Supported metadata value types for the `set` subcommand.
#[derive(Debug, Clone, ValueEnum)]
pub enum ValueType {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F32,
    U64,
    I64,
    F64,
    Bool,
    String,
}

impl ValueType {
    /// Human-readable name used in error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            ValueType::U8 => "u8",
            ValueType::I8 => "i8",
            ValueType::U16 => "u16",
            ValueType::I16 => "i16",
            ValueType::U32 => "u32",
            ValueType::I32 => "i32",
            ValueType::F32 => "f32",
            ValueType::U64 => "u64",
            ValueType::I64 => "i64",
            ValueType::F64 => "f64",
            ValueType::Bool => "bool",
            ValueType::String => "string",
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// Ensure clap's internal invariants are upheld (no duplicate flags, etc.)
    #[test]
    fn cli_validates() {
        Cli::command().debug_assert();
    }

    #[test]
    fn info_subcommand_parses() {
        let cli = Cli::try_parse_from(["gguf-analyzer", "info", "/tmp/model.gguf"]).unwrap();
        assert!(matches!(cli.command, Command::Info(_)));
    }

    #[test]
    fn meta_filter_flag_parses() {
        let cli = Cli::try_parse_from([
            "gguf-analyzer",
            "meta",
            "/tmp/m.gguf",
            "--filter",
            "llama.*",
        ])
        .unwrap();
        if let Command::Meta(args) = cli.command {
            assert_eq!(args.filter.as_deref(), Some("llama.*"));
        } else {
            panic!("expected Meta command");
        }
    }

    #[test]
    fn meta_array_limit_default_is_8() {
        let cli = Cli::try_parse_from(["gguf-analyzer", "meta", "/tmp/m.gguf"]).unwrap();
        if let Command::Meta(args) = cli.command {
            assert_eq!(args.array_limit, 8);
        }
    }

    #[test]
    fn set_dry_run_and_force_default_false() {
        let cli = Cli::try_parse_from([
            "gguf-analyzer",
            "set",
            "/tmp/m.gguf",
            "--key",
            "general.name",
            "--value",
            "MyModel",
            "--type",
            "string",
            "--output",
            "/tmp/out.gguf",
        ])
        .unwrap();
        if let Command::Set(args) = cli.command {
            assert!(!args.dry_run);
            assert!(!args.force);
        }
    }

    #[test]
    fn set_dry_run_flag_sets_true() {
        let cli = Cli::try_parse_from([
            "gguf-analyzer",
            "set",
            "/tmp/m.gguf",
            "--key",
            "general.name",
            "--value",
            "MyModel",
            "--type",
            "string",
            "--output",
            "/tmp/out.gguf",
            "--dry-run",
        ])
        .unwrap();
        if let Command::Set(args) = cli.command {
            assert!(args.dry_run);
        }
    }

    #[test]
    fn remove_requires_output() {
        // Missing --output should fail
        let result =
            Cli::try_parse_from(["gguf-analyzer", "remove", "/tmp/m.gguf", "general.name"]);
        assert!(result.is_err(), "remove without --output should fail");
    }

    #[test]
    fn export_format_default_is_json() {
        let cli = Cli::try_parse_from(["gguf-analyzer", "export", "/tmp/m.gguf"]).unwrap();
        if let Command::Export(args) = cli.command {
            assert!(matches!(args.format, ExportFormat::Json));
        }
    }

    #[test]
    fn value_type_names_are_lowercase_type_strings() {
        assert_eq!(ValueType::U32.type_name(), "u32");
        assert_eq!(ValueType::F32.type_name(), "f32");
        assert_eq!(ValueType::String.type_name(), "string");
        assert_eq!(ValueType::Bool.type_name(), "bool");
    }
}
