//! Integration tests for the `set` subcommand.

mod common;
use gguf_analyzer::{
    cli::{SetArgs, ValueType},
    commands::set::{parse_value, run},
    gguf::ParsedGguf,
};
use gguf_rs_lib::format::metadata::MetadataValue;
use std::path::PathBuf;
use tempfile::NamedTempFile;

// ── parse_value — all scalar types ───────────────────────────────────────────

#[test]
fn parse_value_u8()  { assert_eq!(parse_value("255",  &ValueType::U8).unwrap(),  MetadataValue::U8(255)); }
#[test]
fn parse_value_i8()  { assert_eq!(parse_value("-128", &ValueType::I8).unwrap(),  MetadataValue::I8(-128)); }
#[test]
fn parse_value_u16() { assert_eq!(parse_value("65535",&ValueType::U16).unwrap(), MetadataValue::U16(65535)); }
#[test]
fn parse_value_i16() { assert_eq!(parse_value("-1",   &ValueType::I16).unwrap(), MetadataValue::I16(-1)); }
#[test]
fn parse_value_u32() { assert_eq!(parse_value("42",   &ValueType::U32).unwrap(), MetadataValue::U32(42)); }
#[test]
fn parse_value_i32() { assert_eq!(parse_value("-42",  &ValueType::I32).unwrap(), MetadataValue::I32(-42)); }
#[test]
fn parse_value_u64() { assert_eq!(parse_value("0",    &ValueType::U64).unwrap(), MetadataValue::U64(0)); }
#[test]
fn parse_value_i64() { assert_eq!(parse_value("-1",   &ValueType::I64).unwrap(), MetadataValue::I64(-1)); }
#[test]
fn parse_value_f32() { assert_eq!(parse_value("1.5",  &ValueType::F32).unwrap(), MetadataValue::F32(1.5)); }
#[test]
fn parse_value_f64() { assert_eq!(parse_value("2.5", &ValueType::F64).unwrap(), MetadataValue::F64(2.5)); }
#[test]
fn parse_value_bool_true_aliases() {
    for s in &["true", "1", "yes"] {
        assert_eq!(parse_value(s, &ValueType::Bool).unwrap(), MetadataValue::Bool(true), "alias={s}");
    }
}
#[test]
fn parse_value_bool_false_aliases() {
    for s in &["false", "0", "no"] {
        assert_eq!(parse_value(s, &ValueType::Bool).unwrap(), MetadataValue::Bool(false), "alias={s}");
    }
}
#[test]
fn parse_value_string() {
    assert_eq!(
        parse_value("hello world", &ValueType::String).unwrap(),
        MetadataValue::String("hello world".to_string()),
    );
}

// ── parse_value — error cases ─────────────────────────────────────────────────

#[test]
fn parse_value_u8_overflow_errors() {
    assert!(parse_value("256", &ValueType::U8).is_err());
}
#[test]
fn parse_value_i8_underflow_errors() {
    assert!(parse_value("-129", &ValueType::I8).is_err());
}
#[test]
fn parse_value_bad_bool_errors() {
    assert!(parse_value("maybe", &ValueType::Bool).is_err());
}
#[test]
fn parse_value_bad_u32_errors() {
    assert!(parse_value("not_a_number", &ValueType::U32).is_err());
}

// ── set roundtrip ─────────────────────────────────────────────────────────────

fn output_tmp() -> NamedTempFile {
    NamedTempFile::new().expect("create output temp file")
}

#[test]
fn set_existing_key_roundtrip() {
    let (_src, src_path) = common::minimal_gguf();
    let out_tmp = output_tmp();
    let out_path = out_tmp.path().to_path_buf();

    let args = SetArgs {
        file:    src_path.clone(),
        key:     "general.name".to_string(),
        value:   "new-model-name".to_string(),
        r#type:  ValueType::String,
        output:  out_path.clone(),
        force:   true,
        backup:  false,
        dry_run: false,
    };
    run(&args).expect("set should succeed");

    let gguf2 = ParsedGguf::open(&out_path).expect("open modified file");
    let val = gguf2.metadata.get("general.name").expect("key must exist");
    assert_eq!(
        val,
        &MetadataValue::String("new-model-name".to_string())
    );
}

#[test]
fn set_u32_key_roundtrip() {
    let (_src, src_path) = common::minimal_gguf();
    let out_tmp = output_tmp();
    let out_path = out_tmp.path().to_path_buf();

    let args = SetArgs {
        file:    src_path.clone(),
        key:     "llm.context_length".to_string(),
        value:   "1024".to_string(),
        r#type:  ValueType::U32,
        output:  out_path.clone(),
        force:   true,
        backup:  false,
        dry_run: false,
    };
    run(&args).expect("set should succeed");

    let gguf2 = ParsedGguf::open(&out_path).expect("open modified file");
    let val = gguf2.metadata.get("llm.context_length").expect("key must exist");
    assert_eq!(val, &MetadataValue::U32(1024));
}

#[test]
fn set_missing_key_without_force_errors() {
    let (_src, src_path) = common::minimal_gguf();
    let out_tmp = output_tmp();

    let args = SetArgs {
        file:    src_path.clone(),
        key:     "nonexistent.key".to_string(),
        value:   "value".to_string(),
        r#type:  ValueType::String,
        output:  out_tmp.path().to_path_buf(),
        force:   false,
        backup:  false,
        dry_run: false,
    };
    assert!(run(&args).is_err(), "should fail: key does not exist");
}

#[test]
fn set_missing_key_with_force_creates_key() {
    let (_src, src_path) = common::minimal_gguf();
    let out_tmp = output_tmp();
    let out_path = out_tmp.path().to_path_buf();

    let args = SetArgs {
        file:    src_path.clone(),
        key:     "new.key".to_string(),
        value:   "created".to_string(),
        r#type:  ValueType::String,
        output:  out_path.clone(),
        force:   true,
        backup:  false,
        dry_run: false,
    };
    run(&args).expect("force-create should succeed");

    let gguf2 = ParsedGguf::open(&out_path).expect("open modified file");
    assert!(gguf2.metadata.contains_key("new.key"));
    assert_eq!(
        gguf2.metadata.get("new.key").unwrap(),
        &MetadataValue::String("created".to_string())
    );
}

#[test]
fn set_dry_run_does_not_write_file() {
    let (_src, src_path) = common::minimal_gguf();
    // Point output at a path that does NOT exist yet
    let output = PathBuf::from("/tmp/gguf_test_dry_run_output.gguf");
    // Make sure it doesn't exist from a previous run
    let _ = std::fs::remove_file(&output);

    let args = SetArgs {
        file:    src_path.clone(),
        key:     "general.name".to_string(),
        value:   "dry".to_string(),
        r#type:  ValueType::String,
        output:  output.clone(),
        force:   true,
        backup:  false,
        dry_run: true,
    };
    run(&args).expect("dry-run should return Ok");
    assert!(!output.exists(), "dry-run must not create the output file");
}

#[test]
fn set_backup_renames_existing_output() {
    let (_src, src_path) = common::minimal_gguf();

    // First write — creates the output file.
    let out_tmp = output_tmp();
    let out_path = out_tmp.path().to_path_buf();
    let bak_path = {
        let mut p = out_path.clone().into_os_string();
        p.push(".bak");
        PathBuf::from(p)
    };
    let _ = std::fs::remove_file(&bak_path);

    let first = SetArgs {
        file:    src_path.clone(),
        key:     "general.name".to_string(),
        value:   "v1".to_string(),
        r#type:  ValueType::String,
        output:  out_path.clone(),
        force:   true,
        backup:  false,
        dry_run: false,
    };
    run(&first).expect("first set");
    assert!(out_path.exists());
    assert!(!bak_path.exists(), "no backup expected on first write");

    // Second write with --backup — previous output should move to .bak.
    let second = SetArgs {
        file:    src_path.clone(),
        key:     "general.name".to_string(),
        value:   "v2".to_string(),
        r#type:  ValueType::String,
        output:  out_path.clone(),
        force:   true,
        backup:  true,
        dry_run: false,
    };
    run(&second).expect("second set with backup");

    assert!(out_path.exists(), "new output must exist");
    assert!(bak_path.exists(), "previous output must be preserved as .bak");

    // .bak should still parse as a valid GGUF and contain v1.
    let bak_gguf = ParsedGguf::open(&bak_path).expect("open .bak");
    assert_eq!(
        bak_gguf.metadata.get("general.name").unwrap(),
        &MetadataValue::String("v1".to_string()),
    );

    // New output should contain v2.
    let new_gguf = ParsedGguf::open(&out_path).expect("open new output");
    assert_eq!(
        new_gguf.metadata.get("general.name").unwrap(),
        &MetadataValue::String("v2".to_string()),
    );

    // Cleanup
    let _ = std::fs::remove_file(&bak_path);
}

#[test]
fn set_backup_is_noop_when_output_missing() {
    let (_src, src_path) = common::minimal_gguf();
    let out_tmp = output_tmp();
    let out_path = out_tmp.path().to_path_buf();
    // Ensure the output does not exist (NamedTempFile creates an empty file
    // — explicitly remove it so we exercise the "no prior output" branch).
    let _ = std::fs::remove_file(&out_path);
    let bak_path = {
        let mut p = out_path.clone().into_os_string();
        p.push(".bak");
        PathBuf::from(p)
    };
    let _ = std::fs::remove_file(&bak_path);

    let args = SetArgs {
        file:    src_path.clone(),
        key:     "general.name".to_string(),
        value:   "fresh".to_string(),
        r#type:  ValueType::String,
        output:  out_path.clone(),
        force:   true,
        backup:  true,
        dry_run: false,
    };
    run(&args).expect("set should succeed");
    assert!(out_path.exists());
    assert!(!bak_path.exists(), "no .bak should be created when output didn't exist");
}
