# gguf-analyzer — Requirements

## Overview

`gguf-analyzer` is a Rust command-line tool for inspecting and editing the metadata sections of **GGUF** model files (the binary format used by llama.cpp, Ollama, and the broader GGML ecosystem). It operates entirely through memory-mapped I/O so that multi-gigabyte weight files are never loaded into the heap.

---

## User Stories

### US-01 · Quick file overview (`info`)
> **As a** ML engineer,  
> **I want to** get a one-glance summary of a GGUF file,  
> **so that** I can confirm the version, model architecture, tensor count, and alignment without writing any code.

### US-02 · Browse metadata key-value pairs (`meta`)
> **As a** researcher,  
> **I want to** list all metadata fields in a GGUF file in a readable table,  
> **so that** I can understand what information is stored alongside the weights.

### US-03 · Filter metadata by pattern (`meta --filter`)
> **As a** researcher,  
> **I want to** filter displayed metadata entries using a glob pattern (e.g. `llama.*`),  
> **so that** I can focus on a specific namespace without scrolling through hundreds of keys.

### US-04 · Browse tensor inventory (`tensors`)
> **As a** ML engineer,  
> **I want to** list all tensors with their name, shape, quantisation type, byte offset, and size,  
> **so that** I can audit what layers are present and how they are quantised.

### US-05 · Filter tensors by pattern (`tensors --filter`)
> **As a** ML engineer,  
> **I want to** filter the tensor table with a glob pattern (e.g. `blk.0.*`),  
> **so that** I can inspect a subset of the model without reviewing every entry.

### US-06 · Edit a metadata value (`set`)
> **As a** fine-tuner,  
> **I want to** change a metadata value (e.g. `general.name`) and write the result to a new file,  
> **so that** I can tag a customised model without re-quantising the weights.

### US-07 · Preview a metadata change without writing (`set --dry-run`)
> **As a** cautious engineer,  
> **I want to** preview the effect of a `set` operation before committing to disk,  
> **so that** I can verify the key, type, and new value are correct.

### US-08 · Remove a metadata key (`remove`)
> **As a** distribution packager,  
> **I want to** delete a metadata key and write the result to a new file,  
> **so that** I can strip internal tracking fields before publishing a model.

### US-09 · Preview a metadata removal without writing (`remove --dry-run`)
> **As a** distribution packager,  
> **I want to** confirm which key will be removed before writing the output,  
> **so that** I avoid accidental data loss.

### US-10 · Export metadata to JSON (`export --format json`)
> **As a** tooling developer,  
> **I want to** export all metadata to a structured JSON file,  
> **so that** I can feed it into downstream scripts and dashboards.

### US-11 · Export metadata to Markdown (`export --format markdown`)
> **As a** model author,  
> **I want to** export metadata as a Markdown table,  
> **so that** I can paste it directly into a model card or README.

### US-12 · Export metadata to CSV (`export --format csv`)
> **As a** data analyst,  
> **I want to** export metadata to a CSV file,  
> **so that** I can open it in a spreadsheet or import it into a database.

### US-13 · Shell completions (`completions`)
> **As a** power user,  
> **I want to** generate shell completion scripts for Bash, Zsh, Fish, or PowerShell,  
> **so that** I can tab-complete subcommands and flags in my terminal.

### US-14 · Alignment-safe writes
> **As a** ML engineer,  
> **I want** all write operations (`set`, `remove`) to correctly pad the header section to the model's declared `general.alignment` (default 32 bytes),  
> **so that** the output file remains valid and loadable by llama.cpp and Ollama.

### US-15 · Overwrite protection (`--force`)
> **As a** careful engineer,  
> **I want** the tool to refuse to overwrite an existing output file unless `--force` is passed,  
> **so that** I never accidentally clobber a good model file.

---

## Acceptance Criteria

### AC-01 · `info` subcommand
- [ ] Prints GGUF magic and version number.
- [ ] Prints tensor count and metadata key-value count.
- [ ] Prints the declared `general.alignment` (falls back to 32 if absent).
- [ ] Prints the total file size in human-readable form (e.g. `3.92 GiB`).
- [ ] Prints the byte offset at which tensor data begins.
- [ ] Output fits within the current terminal width (uses `terminal_size`).
- [ ] Returns exit code 0 on success, non-zero on error.

### AC-02 · `meta` subcommand (table view)
- [ ] Displays a two-column table: **Key** and **Value**.
- [ ] Array values are truncated to `--array-limit` elements (default 8) with a `[… N more]` suffix.
- [ ] Table columns are sized to fit the terminal width.
- [ ] With `--filter <GLOB>`, only keys matching the pattern (case-insensitive) are shown.
- [ ] If no keys match the filter, prints an informational message rather than an empty table.
- [ ] Returns exit code 0 on success.

### AC-03 · `tensors` subcommand (table view)
- [ ] Displays columns: **Name**, **Shape**, **Type**, **Offset**, **Size**.
- [ ] Shape is formatted as `[d0, d1, …]`.
- [ ] Size is in human-readable bytes (e.g. `512 MiB`).
- [ ] With `--filter <GLOB>`, only tensor names matching the pattern (case-insensitive) are shown.
- [ ] Table columns fit within terminal width.
- [ ] Returns exit code 0 on success.

### AC-04 · `set` subcommand
- [ ] Accepts `--key`, `--value`, `--type`, `--output` arguments.
- [ ] Supported types: `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `f32`, `u64`, `i64`, `f64`, `bool`, `string`.
- [ ] Returns an error if the key does not exist and `--force` is not passed.
- [ ] With `--force`, a non-existent key is **created**.
- [ ] With `--dry-run`, prints what would change but writes nothing.
- [ ] Writes a new GGUF file to `--output`; refuses to overwrite unless `--force`.
- [ ] The output file passes GGUF magic + version validation.
- [ ] Padding between header and tensor data satisfies `general.alignment`.
- [ ] Tensor data bytes are bit-for-bit identical to the source file.
- [ ] Returns exit code 0 on success, non-zero if key not found (without `--force`).

### AC-05 · `remove` subcommand
- [ ] Accepts `--key`, `--output` arguments.
- [ ] Returns an error if the key does not exist.
- [ ] With `--dry-run`, prints what would be removed but writes nothing.
- [ ] Writes a new GGUF file to `--output`; refuses to overwrite unless `--force`.
- [ ] The output file passes GGUF magic + version validation.
- [ ] Padding between header and tensor data satisfies `general.alignment`.
- [ ] Tensor data bytes are bit-for-bit identical to the source file.
- [ ] Returns exit code 0 on success, non-zero if key not found.

### AC-06 · `export` subcommand
- [ ] Supports `--format json` (default), `--format markdown`, `--format csv`.
- [ ] Without `--output`, writes to stdout.
- [ ] With `--output <FILE>`, writes to the specified path.
- [ ] JSON output is valid, parseable JSON with all metadata keys and values.
- [ ] Markdown output is a valid GitHub-flavoured Markdown table.
- [ ] CSV output has a header row (`key,type,value`) and one row per metadata entry.
- [ ] Array values are truncated to `--array-limit` elements in all formats.
- [ ] Returns exit code 0 on success.

### AC-07 · `completions` subcommand
- [ ] Supports `bash`, `zsh`, `fish`, `powershell`, `elvish` shells.
- [ ] Writes the generated completion script to stdout.
- [ ] The script is syntactically valid for the target shell.

### AC-08 · Error messages
- [ ] Every error includes the affected file path or metadata key.
- [ ] Errors are printed to stderr.
- [ ] The process exits with a non-zero code on any error.

### AC-09 · Memory and performance
- [ ] The tool opens GGUF files via memory-map (`memmap2`); `fs::read` is never used for the full file.
- [ ] Peak RSS does not scale with the size of the tensor data section during read-only operations.

### AC-10 · Build and test
- [ ] `cargo build --release` succeeds with zero errors and zero `deny(warnings)` violations.
- [ ] `cargo test` passes all unit tests.
- [ ] No `unsafe` blocks outside of the single `memmap2::Mmap::map` call.

---

## Non-functional Requirements

| ID | Requirement |
|----|-------------|
| NFR-01 | Written in Rust, edition 2024, targeting stable toolchain. |
| NFR-02 | All public-facing errors use `anyhow` context chains for actionable messages. |
| NFR-03 | Internal typed errors use `thiserror`; never `unwrap()` in production paths. |
| NFR-04 | Table output respects terminal width via `terminal_size`; falls back to 80 columns. |
| NFR-05 | Glob filter matching is case-insensitive. |
| NFR-06 | Default `--array-limit` is 8; the flag is present on `meta` and `export`. |
| NFR-07 | `--dry-run` produces output to stdout and exits 0 without modifying any file. |
| NFR-08 | The binary is named `gguf-analyzer` (set via `[[bin]]` in `Cargo.toml`). |

---

## Tech Stack

| Concern | Crate |
|---------|-------|
| GGUF I/O | `gguf-rs-lib 0.2` |
| Memory-mapping | `memmap2 0.9` (via gguf-rs-lib `mmap` feature + direct use) |
| CLI parsing | `clap 4` (derive feature) |
| Shell completions | `clap_complete 4` |
| Typed errors | `thiserror 2` |
| Error context | `anyhow 1` |
| JSON export | `serde_json 1` + `serde 1` |
| Table rendering | `tabled 0.17` |
| Terminal width | `terminal_size 0.4` |
| Glob filtering | `glob 0.3` |
