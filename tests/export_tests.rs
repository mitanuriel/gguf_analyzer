//! Integration tests for the `export` subcommand.

mod common;
use gguf_analyzer::{
    cli::{ExportArgs, ExportFormat},
    commands::export::run,
};

fn run_export(format: ExportFormat, path: &std::path::Path) -> String {
    // Capture by writing to a temp file
    let out_tmp = tempfile::NamedTempFile::new().unwrap();
    let out_path = out_tmp.path().to_path_buf();

    let args = ExportArgs {
        file:        path.to_path_buf(),
        output:      Some(out_path.clone()),
        format,
        array_limit: 8,
    };
    run(&args).expect("export should succeed");
    std::fs::read_to_string(&out_path).expect("read export output")
}

// ── JSON ──────────────────────────────────────────────────────────────────────

#[test]
fn export_json_is_valid_json() {
    let (_tmp, path) = common::minimal_gguf();
    let output = run_export(ExportFormat::Json, &path);
    serde_json::from_str::<serde_json::Value>(&output).expect("must be valid JSON");
}

#[test]
fn export_json_contains_all_three_keys() {
    let (_tmp, path) = common::minimal_gguf();
    let output = run_export(ExportFormat::Json, &path);
    let v: serde_json::Value = serde_json::from_str(&output).unwrap();
    let obj = v.as_object().unwrap();

    assert!(obj.contains_key("general.architecture"), "missing general.architecture");
    assert!(obj.contains_key("general.name"),         "missing general.name");
    assert!(obj.contains_key("llm.context_length"),   "missing llm.context_length");
}

#[test]
fn export_json_entry_has_type_and_value_fields() {
    let (_tmp, path) = common::minimal_gguf();
    let output = run_export(ExportFormat::Json, &path);
    let v: serde_json::Value = serde_json::from_str(&output).unwrap();
    let entry = &v["general.name"];

    assert_eq!(entry["type"].as_str().unwrap(),  "string");
    assert_eq!(entry["value"].as_str().unwrap(), "test-model");
}

#[test]
fn export_json_u32_value_is_correct() {
    let (_tmp, path) = common::minimal_gguf();
    let output = run_export(ExportFormat::Json, &path);
    let v: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(v["llm.context_length"]["type"].as_str().unwrap(),  "u32");
    assert_eq!(v["llm.context_length"]["value"].as_str().unwrap(), "512");
}

#[test]
fn export_json_is_sorted_by_key() {
    let (_tmp, path) = common::minimal_gguf();
    let output = run_export(ExportFormat::Json, &path);
    let v: serde_json::Value = serde_json::from_str(&output).unwrap();
    let keys: Vec<&str> = v.as_object().unwrap().keys().map(|s| s.as_str()).collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "JSON output must be sorted alphabetically");
}

// ── Markdown ──────────────────────────────────────────────────────────────────

#[test]
fn export_markdown_starts_with_header() {
    let (_tmp, path) = common::minimal_gguf();
    let output = run_export(ExportFormat::Markdown, &path);
    assert!(output.starts_with("| Key |"), "must start with markdown table header");
}

#[test]
fn export_markdown_contains_separator_row() {
    let (_tmp, path) = common::minimal_gguf();
    let output = run_export(ExportFormat::Markdown, &path);
    assert!(output.contains("|-----|"), "must contain separator row");
}

#[test]
fn export_markdown_contains_all_keys() {
    let (_tmp, path) = common::minimal_gguf();
    let output = run_export(ExportFormat::Markdown, &path);
    assert!(output.contains("`general.architecture`"));
    assert!(output.contains("`general.name`"));
    assert!(output.contains("`llm.context_length`"));
}

#[test]
fn export_markdown_contains_values() {
    let (_tmp, path) = common::minimal_gguf();
    let output = run_export(ExportFormat::Markdown, &path);
    assert!(output.contains("test-model"),  "must include string value");
    assert!(output.contains("test-arch"),   "must include architecture value");
    assert!(output.contains("512"),         "must include context length value");
}

// ── CSV ───────────────────────────────────────────────────────────────────────

#[test]
fn export_csv_starts_with_header() {
    let (_tmp, path) = common::minimal_gguf();
    let output = run_export(ExportFormat::Csv, &path);
    assert!(output.starts_with("key,type,value\n"), "must start with CSV header");
}

#[test]
fn export_csv_contains_all_keys() {
    let (_tmp, path) = common::minimal_gguf();
    let output = run_export(ExportFormat::Csv, &path);
    assert!(output.contains("general.architecture"));
    assert!(output.contains("general.name"));
    assert!(output.contains("llm.context_length"));
}

#[test]
fn export_csv_contains_values() {
    let (_tmp, path) = common::minimal_gguf();
    let output = run_export(ExportFormat::Csv, &path);
    assert!(output.contains("test-model"), "must include string value");
    assert!(output.contains("512"),        "must include numeric value");
}

#[test]
fn export_csv_values_are_quoted() {
    let (_tmp, path) = common::minimal_gguf();
    let output = run_export(ExportFormat::Csv, &path);
    // Each value field is wrapped in double-quotes
    assert!(output.contains("\"test-model\""), "string values must be double-quoted");
}
