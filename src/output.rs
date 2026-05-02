//! Helpers for resolving optional output paths.

use std::path::{Path, PathBuf};

/// Resolve an optional `--output` path.
///
/// If `output` is `Some`, returns it as-is.
/// If `output` is `None`, derives a default next to the source file:
///   `<dir>/<stem><suffix>.gguf`
///
/// # Example
/// ```
/// # use std::path::PathBuf;
/// # use gguf_analyzer::output::resolve_output;
/// let out = resolve_output(
///     &PathBuf::from("/home/user/models/model.gguf"),
///     None,
///     "-sampled",
/// );
/// assert_eq!(out, PathBuf::from("/home/user/models/model-sampled.gguf"));
/// ```
pub fn resolve_output(source: &Path, output: Option<&Path>, suffix: &str) -> PathBuf {
    if let Some(p) = output {
        return p.to_path_buf();
    }

    let parent = source.parent().unwrap_or_else(|| Path::new("."));
    let stem = source
        .file_stem()
        .unwrap_or(source.as_os_str())
        .to_string_lossy();

    parent.join(format!("{stem}{suffix}.gguf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_sibling_with_suffix() {
        let src = PathBuf::from("/models/qwen/Qwen3-0.6B-Q8_0.gguf");
        let out = resolve_output(&src, None, "-sampled");
        assert_eq!(
            out,
            PathBuf::from("/models/qwen/Qwen3-0.6B-Q8_0-sampled.gguf")
        );
    }

    #[test]
    fn explicit_output_wins() {
        let src = PathBuf::from("/models/model.gguf");
        let explicit = PathBuf::from("/tmp/custom.gguf");
        let out = resolve_output(&src, Some(&explicit), "-sampled");
        assert_eq!(out, explicit);
    }

    #[test]
    fn works_with_bare_filename() {
        let src = PathBuf::from("model.gguf");
        let out = resolve_output(&src, None, "-modified");
        assert_eq!(out, PathBuf::from("model-modified.gguf"));
    }
}
