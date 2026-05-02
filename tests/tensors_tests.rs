//! Integration tests for the `tensors` subcommand.

mod common;

use gguf_analyzer::{display::format_shape, gguf::ParsedGguf};
use glob::Pattern;

// ── Fixture tensor assertions ─────────────────────────────────────────────────

#[test]
fn tensors_count_is_one() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");
    assert_eq!(gguf.tensor_infos.len(), 1);
}

#[test]
fn tensors_name_is_correct() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");
    assert_eq!(gguf.tensor_infos[0].name, "token_embd.weight");
}

#[test]
fn tensors_shape_dims_are_4_and_8() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");
    let dims = gguf.tensor_infos[0].shape.dims();
    assert_eq!(dims, &[4u64, 8u64]);
}

#[test]
fn tensors_element_count_is_32() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");
    let elems: u64 = gguf.tensor_infos[0].shape.dims().iter().product();
    assert_eq!(elems, 32);
}

#[test]
fn tensors_data_offset_is_accessible() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");
    let _ = gguf.tensor_infos[0].data_offset;
}

// ── format_shape helper ───────────────────────────────────────────────────────

#[test]
fn format_shape_2d() {
    assert_eq!(format_shape(&[4, 8]), "[4 × 8]");
}

#[test]
fn format_shape_1d() {
    assert_eq!(format_shape(&[128]), "[128]");
}

#[test]
fn format_shape_3d() {
    assert_eq!(format_shape(&[2, 3, 4]), "[2 × 3 × 4]");
}

#[test]
fn format_shape_empty() {
    assert_eq!(format_shape(&[]), "[]");
}

// ── Glob filtering ────────────────────────────────────────────────────────────

#[test]
fn tensor_glob_weight_matches_one() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");
    let pat = Pattern::new("*weight*").unwrap();

    let matches: Vec<_> = gguf
        .tensor_infos
        .iter()
        .filter(|t| pat.matches(&t.name.to_lowercase()))
        .collect();

    assert_eq!(matches.len(), 1);
}

#[test]
fn tensor_glob_missing_returns_empty() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");
    let pat = Pattern::new("attn.*").unwrap();

    let matches: Vec<_> = gguf
        .tensor_infos
        .iter()
        .filter(|t| pat.matches(&t.name.to_lowercase()))
        .collect();

    assert!(matches.is_empty());
}

#[test]
fn tensor_glob_exact_name_matches() {
    let (_tmp, path) = common::minimal_gguf();
    let gguf = ParsedGguf::open(&path).expect("open fixture");
    let pat = Pattern::new("token_embd.weight").unwrap();

    let matches: Vec<_> = gguf
        .tensor_infos
        .iter()
        .filter(|t| pat.matches(&t.name.to_lowercase()))
        .collect();

    assert_eq!(matches.len(), 1);
}
