//! Integration tests for the `meta` subcommand.

mod common;
use gguf_analyzer::{
    display::{format_type, format_value},
    gguf::ParsedGguf,
};
use gguf_rs_lib::format::metadata::MetadataValue;
use glob::Pattern;

// ── Fixture metadata assertions ───────────────────────────────────────────────

#[test]
fn meta_all_keys_present() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");

    assert!(gguf.metadata.contains_key("general.architecture"));
    assert!(gguf.metadata.contains_key("general.name"));
    assert!(gguf.metadata.contains_key("llm.context_length"));
}

#[test]
fn meta_string_value_roundtrips() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");

    let val = gguf.metadata.get("general.name").expect("key exists");
    assert_eq!(format_value(val, 8), "test-model");
    assert_eq!(format_type(val), "string");
}

#[test]
fn meta_u32_value_roundtrips() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");

    let val = gguf.metadata.get("llm.context_length").expect("key exists");
    assert_eq!(format_value(val, 8), "512");
    assert_eq!(format_type(val), "u32");
}

// ── Glob filtering ────────────────────────────────────────────────────────────

#[test]
fn meta_glob_filter_matches_prefix() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");
    let pat = Pattern::new("general.*").unwrap();

    let matches: Vec<_> = gguf
        .metadata
        .iter()
        .filter(|(k, _)| pat.matches(&k.to_lowercase()))
        .collect();

    assert_eq!(matches.len(), 2, "general.* matches 2 keys");
}

#[test]
fn meta_glob_filter_matches_exact() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");
    let pat = Pattern::new("general.name").unwrap();

    let matches: Vec<_> = gguf
        .metadata
        .iter()
        .filter(|(k, _)| pat.matches(&k.to_lowercase()))
        .collect();

    assert_eq!(matches.len(), 1);
}

#[test]
fn meta_glob_filter_no_match_returns_empty() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");
    let pat = Pattern::new("tokenizer.*").unwrap();

    let matches: Vec<_> = gguf
        .metadata
        .iter()
        .filter(|(k, _)| pat.matches(&k.to_lowercase()))
        .collect();

    assert!(matches.is_empty());
}

#[test]
fn meta_glob_is_case_insensitive() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");
    // Simulating the CLI behaviour: the pattern is lowercased before compilation.
    let pat = Pattern::new(&"GENERAL.*".to_lowercase()).unwrap();

    let matches: Vec<_> = gguf
        .metadata
        .iter()
        .filter(|(k, _)| pat.matches(&k.to_lowercase()))
        .collect();

    assert_eq!(matches.len(), 2);
}

// ── format_value / format_type matrix ────────────────────────────────────────

#[test]
fn format_type_covers_all_scalars() {
    assert_eq!(format_type(&MetadataValue::U8(0)),     "u8");
    assert_eq!(format_type(&MetadataValue::I8(0)),     "i8");
    assert_eq!(format_type(&MetadataValue::U16(0)),    "u16");
    assert_eq!(format_type(&MetadataValue::I16(0)),    "i16");
    assert_eq!(format_type(&MetadataValue::U32(0)),    "u32");
    assert_eq!(format_type(&MetadataValue::I32(0)),    "i32");
    assert_eq!(format_type(&MetadataValue::F32(0.0)),  "f32");
    assert_eq!(format_type(&MetadataValue::U64(0)),    "u64");
    assert_eq!(format_type(&MetadataValue::I64(0)),    "i64");
    assert_eq!(format_type(&MetadataValue::F64(0.0)),  "f64");
    assert_eq!(format_type(&MetadataValue::Bool(true)), "bool");
    assert_eq!(format_type(&MetadataValue::String(String::new())), "string");
}

#[test]
fn format_value_covers_all_scalars() {
    assert_eq!(format_value(&MetadataValue::U8(255),     8), "255");
    assert_eq!(format_value(&MetadataValue::I8(-1),      8), "-1");
    assert_eq!(format_value(&MetadataValue::U16(1000),   8), "1000");
    assert_eq!(format_value(&MetadataValue::I16(-500),   8), "-500");
    assert_eq!(format_value(&MetadataValue::U32(99999),  8), "99999");
    assert_eq!(format_value(&MetadataValue::I32(-99999), 8), "-99999");
    assert_eq!(format_value(&MetadataValue::U64(u64::MAX), 8), u64::MAX.to_string());
    assert_eq!(format_value(&MetadataValue::I64(i64::MIN), 8), i64::MIN.to_string());
    assert_eq!(format_value(&MetadataValue::Bool(false), 8), "false");
    assert_eq!(format_value(&MetadataValue::Bool(true),  8), "true");
    assert_eq!(
        format_value(&MetadataValue::String("hello world".to_string()), 8),
        "hello world"
    );
}
