//! Integration tests for the `remove` subcommand.

mod common;
use gguf_analyzer::{cli::RemoveArgs, commands::remove::run, gguf::ParsedGguf};
use std::path::PathBuf;
use tempfile::NamedTempFile;

fn output_tmp() -> NamedTempFile {
    NamedTempFile::new().expect("create output temp file")
}

// ── roundtrip ─────────────────────────────────────────────────────────────────

#[test]
fn remove_key_disappears_from_output() {
    let (_src, src_path) = common::minimal_gguf();
    let out_tmp = output_tmp();
    let out_path = out_tmp.path().to_path_buf();

    let args = RemoveArgs {
        file: src_path.clone(),
        key: "llm.context_length".to_string(),
        output: out_path.clone(),
        force: true,
        backup: false,
        dry_run: false,
    };
    run(&args).expect("remove should succeed");

    let gguf2 = ParsedGguf::open(&out_path).expect("open modified file");
    assert!(
        !gguf2.metadata.contains_key("llm.context_length"),
        "removed key must be absent"
    );
}

#[test]
fn remove_decrements_metadata_count() {
    let (_src, src_path) = common::minimal_gguf();
    let original_count = ParsedGguf::open(&src_path).unwrap().metadata.len();

    let out_tmp = output_tmp();
    let out_path = out_tmp.path().to_path_buf();

    let args = RemoveArgs {
        file: src_path.clone(),
        key: "general.name".to_string(),
        output: out_path.clone(),
        force: true,
        backup: false,
        dry_run: false,
    };
    run(&args).expect("remove should succeed");

    let gguf2 = ParsedGguf::open(&out_path).expect("open modified file");
    assert_eq!(gguf2.metadata.len(), original_count - 1);
}

#[test]
fn remove_other_keys_are_preserved() {
    let (_src, src_path) = common::minimal_gguf();
    let out_tmp = output_tmp();
    let out_path = out_tmp.path().to_path_buf();

    let args = RemoveArgs {
        file: src_path.clone(),
        key: "llm.context_length".to_string(),
        output: out_path.clone(),
        force: true,
        backup: false,
        dry_run: false,
    };
    run(&args).expect("remove should succeed");

    let gguf2 = ParsedGguf::open(&out_path).expect("open modified file");
    assert!(gguf2.metadata.contains_key("general.architecture"));
    assert!(gguf2.metadata.contains_key("general.name"));
}

#[test]
fn remove_missing_key_errors() {
    let (_src, src_path) = common::minimal_gguf();
    let out_tmp = output_tmp();

    let args = RemoveArgs {
        file: src_path.clone(),
        key: "does.not.exist".to_string(),
        output: out_tmp.path().to_path_buf(),
        force: false,
        backup: false,
        dry_run: false,
    };
    assert!(run(&args).is_err(), "removing absent key must return Err");
}

#[test]
fn remove_dry_run_does_not_write_file() {
    let (_src, src_path) = common::minimal_gguf();
    let out_path = std::path::PathBuf::from("/tmp/gguf_test_remove_dry_run.gguf");
    let _ = std::fs::remove_file(&out_path);

    let args = RemoveArgs {
        file: src_path.clone(),
        key: "general.name".to_string(),
        output: out_path.clone(),
        force: true,
        backup: false,
        dry_run: true,
    };
    run(&args).expect("dry-run should return Ok");
    assert!(
        !out_path.exists(),
        "dry-run must not create the output file"
    );
}

/// Regression: in-place remove (--output == source) with --backup must
/// succeed by redirecting the reader to the renamed `.bak` file.
#[test]
fn remove_inplace_with_backup_succeeds() {
    let (src, src_path) = common::minimal_gguf();
    let bak_path = {
        let mut p = src_path.clone().into_os_string();
        p.push(".bak");
        PathBuf::from(p)
    };
    let _ = std::fs::remove_file(&bak_path);

    let args = RemoveArgs {
        file: src_path.clone(),
        key: "general.name".to_string(),
        output: src_path.clone(),
        force: true,
        backup: true,
        dry_run: false,
    };
    run(&args).expect("in-place remove with --backup must succeed");

    assert!(src_path.exists(), "new file at original path");
    assert!(bak_path.exists(), "previous version preserved as .bak");

    let new_gguf = ParsedGguf::open(&src_path).expect("open new output");
    assert!(
        !new_gguf.metadata.contains_key("general.name"),
        "removed key must be absent from new output"
    );

    let bak_gguf = ParsedGguf::open(&bak_path).expect("open .bak");
    assert!(
        bak_gguf.metadata.contains_key("general.name"),
        "key must still be present in .bak (the pre-remove snapshot)"
    );

    drop(src);
    let _ = std::fs::remove_file(&bak_path);
}
