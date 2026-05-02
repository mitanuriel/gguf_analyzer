//! Library interface for `gguf-analyzer`.
//!
//! Exposes the core modules so integration tests (and downstream crates) can
//! call display helpers and open GGUF files without going through the binary.

pub mod cli;
pub mod commands;
pub mod display;
pub mod error;
pub mod gguf;
