//! Integration tests for the `info` subcommand.

mod common;
use gguf_analyzer::{
    display::format_bytes,
    gguf::ParsedGguf,
};

/// ParsedGguf::open succeeds on the fixture and returns expected field values.
#[test]
fn info_opens_fixture() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");

    assert_eq!(gguf.version, 3, "GGUF version should be 3");
    assert_eq!(gguf.tensor_count, 1, "fixture has exactly 1 tensor");
    assert_eq!(gguf.metadata.len(), 3, "fixture has 3 metadata entries");
    assert_eq!(gguf.alignment, 32, "should fall back to default alignment 32");
    assert!(gguf.file_size > 0, "file size must be non-zero");
}

/// The tensor data offset is always > 0 (header + metadata bytes).
#[test]
fn info_tensor_data_offset_is_nonzero() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");
    assert!(gguf.tensor_data_offset > 0);
}

/// format_bytes is used in the info table; verify it at common sizes.
#[test]
fn info_format_bytes_boundaries() {
    assert_eq!(format_bytes(0),             "0 B");
    assert_eq!(format_bytes(1023),          "1023 B");
    assert_eq!(format_bytes(1024),          "1.00 KiB");
    assert_eq!(format_bytes(1_048_576),     "1.00 MiB");
    assert_eq!(format_bytes(1_073_741_824), "1.00 GiB");
}

/// Opening a non-existent path returns an error (not a panic).
#[test]
fn info_missing_file_returns_err() {
    let result = ParsedGguf::open("/no/such/file.gguf");
    assert!(result.is_err());
}
