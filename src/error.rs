//! Application-level error types.
//!
//! [`AppError`] is the single typed-error enum used throughout the crate.
//! All variants carry enough context so the user knows *which* file or *which*
//! key caused the problem.  Higher-level code wraps these with
//! [`anyhow::Context`] to add extra call-site information.

use std::path::PathBuf;
use thiserror::Error;

/// All errors that can be produced by `gguf-analyzer`.
///
/// Some variants (`GgufParse`, `TypeParse`, `UnknownExportFormat`) are defined
/// for completeness and future use; they may not be constructed by the current
/// code paths.
#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum AppError {
    // ── I/O ──────────────────────────────────────────────────────────────────
    /// A filesystem or memory-mapping operation failed.
    #[error("I/O error on '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    // ── GGUF parsing ─────────────────────────────────────────────────────────
    /// The file is not a valid GGUF file or is structurally corrupt.
    #[error("Failed to parse GGUF file '{path}': {message}")]
    GgufParse { path: PathBuf, message: String },

    /// The library returned an error while reading/writing the GGUF format.
    #[error("GGUF library error on '{path}': {source}")]
    GgufLib {
        path: PathBuf,
        #[source]
        source: gguf_rs_lib::GGUFError,
    },

    // ── Metadata key/value ───────────────────────────────────────────────────
    /// A key that was expected to exist is absent from the metadata.
    #[error("Metadata key '{key}' not found in '{path}'")]
    KeyNotFound { key: String, path: PathBuf },

    /// The string supplied by the user could not be parsed as the requested
    /// metadata value type.
    #[error("Cannot parse '{value}' as type '{type_name}': {reason}")]
    TypeParse {
        value: String,
        type_name: &'static str,
        reason: String,
    },

    // ── Output file safety ───────────────────────────────────────────────────
    /// The output path already exists and `--force` was not supplied.
    #[error(
        "Output file '{path}' already exists. \
         Pass --force to overwrite, or choose a different --output path."
    )]
    OutputExists { path: PathBuf },

    // ── Export ───────────────────────────────────────────────────────────────
    /// An unsupported export format was requested.
    #[error("Unknown export format '{format}'. Supported: json, markdown, csv")]
    UnknownExportFormat { format: String },
}

impl AppError {
    /// Convenience constructor: wrap a [`std::io::Error`] with file context.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        AppError::Io {
            path: path.into(),
            source,
        }
    }

    /// Convenience constructor: wrap a [`gguf_rs_lib::GGUFError`] with file
    /// context.
    pub fn gguf_lib(path: impl Into<PathBuf>, source: gguf_rs_lib::GGUFError) -> Self {
        AppError::GgufLib {
            path: path.into(),
            source,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn key_not_found_message_contains_key_and_path() {
        let err = AppError::KeyNotFound {
            key: "general.name".to_string(),
            path: PathBuf::from("/tmp/model.gguf"),
        };
        let msg = err.to_string();
        assert!(msg.contains("general.name"), "message: {msg}");
        assert!(msg.contains("/tmp/model.gguf"), "message: {msg}");
    }

    #[test]
    fn type_parse_message_contains_value_and_type() {
        let err = AppError::TypeParse {
            value: "not-a-number".to_string(),
            type_name: "u32",
            reason: "invalid digit found".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("not-a-number"), "message: {msg}");
        assert!(msg.contains("u32"), "message: {msg}");
        assert!(msg.contains("invalid digit"), "message: {msg}");
    }

    #[test]
    fn output_exists_message_contains_path() {
        let err = AppError::OutputExists {
            path: PathBuf::from("/tmp/out.gguf"),
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/out.gguf"), "message: {msg}");
        assert!(msg.contains("--force"), "should mention --force: {msg}");
    }

    #[test]
    fn io_error_carries_path() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let err = AppError::io(Path::new("/nonexistent.gguf"), io);
        let msg = err.to_string();
        assert!(msg.contains("/nonexistent.gguf"), "message: {msg}");
        assert!(msg.contains("no such file"), "message: {msg}");
    }

    #[test]
    fn unknown_export_format_lists_supported() {
        let err = AppError::UnknownExportFormat {
            format: "yaml".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("yaml"), "message: {msg}");
        assert!(msg.contains("json"), "should list supported formats: {msg}");
    }

    #[test]
    fn gguf_parse_message_contains_path_and_message() {
        let err = AppError::GgufParse {
            path: PathBuf::from("/models/llama.gguf"),
            message: "bad magic".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/models/llama.gguf"), "message: {msg}");
        assert!(msg.contains("bad magic"), "message: {msg}");
    }
}
