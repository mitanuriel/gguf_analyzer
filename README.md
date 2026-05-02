# gguf-analyzer

[![CI](https://github.com/mitanuriel/gguf_analyzer/actions/workflows/ci.yml/badge.svg)](https://github.com/mitanuriel/gguf_analyzer/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/mitanuriel/gguf_analyzer?include_prereleases&sort=semver)](https://github.com/mitanuriel/gguf_analyzer/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/mitanuriel/gguf_analyzer/total)](https://github.com/mitanuriel/gguf_analyzer/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust 2024](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)

A fast, colourful CLI tool for exploring and editing the metadata of
[GGUF](https://github.com/ggerganov/ggml/blob/master/docs/gguf.md) model files —
the format used by llama.cpp, Ollama, LM Studio, and most local AI tools.

Reads files via **memory-map** so even 50 GB models open instantly.
Writes are always to a **new output file** — the source is never touched.

---

## Screenshots

### `info` — file overview

<img width="650" height="154" alt="Screenshot 2026-05-02 at 06 42 26" src="https://github.com/user-attachments/assets/66fad96c-6249-457e-93d3-017a94e0a1f2" />


### `meta` — metadata table
<img width="661" height="421" alt="Screenshot 2026-05-02 at 06 42 48" src="https://github.com/user-attachments/assets/278604a2-1776-44d9-a006-374c60b1f046" />


### `tensors` — tensor inventory

<img width="580" height="282" alt="Screenshot 2026-05-02 at 06 28 46" src="https://github.com/user-attachments/assets/d8f695ff-941f-4cac-9935-a636099a905d" />

### `fetch from HuggingFace` — downloading from Hugging Face repo
<img width="255" height="86" alt="Screenshot 2026-05-02 at 07 25 16" src="https://github.com/user-attachments/assets/238dc9ee-61bf-4422-9714-bcd30724b6ae" />

### `model card from HuggingFace` — moel data and recommended sampling parameters
<img width="403" height="273" alt="Screenshot 2026-05-02 at 07 24 33" src="https://github.com/user-attachments/assets/d393da92-4355-4847-b81b-dc52f99482b4" />

<img width="642" height="655" alt="Screenshot 2026-05-02 at 07 26 33" src="https://github.com/user-attachments/assets/9607956b-b3cf-49e2-9961-eb09f92178af" />






---

## Installation

### Option A — One-line install script (easiest, no Rust required)

**macOS / Linux:**

```bash
curl -LsSf https://github.com/mitanuriel/gguf_analyzer/releases/latest/download/gguf_analyzer-installer.sh | sh
```

**Windows (PowerShell):**

```powershell
powershell -c "irm https://github.com/mitanuriel/gguf_analyzer/releases/latest/download/gguf_analyzer-installer.ps1 | iex"
```

The script downloads the right binary for your platform, verifies checksums, and drops `gguf-analyzer` into `~/.cargo/bin/` (or `%USERPROFILE%\.cargo\bin\` on Windows). Add that directory to your `PATH` if it isn't already.

---

### Option B — Download a pre-built binary manually

Go to the [**Releases page**](https://github.com/mitanuriel/gguf_analyzer/releases) and
download the archive for your platform:

| Platform | File to download |
|---|---|
| macOS (Apple Silicon M1/M2/M3) | `gguf_analyzer-aarch64-apple-darwin.tar.xz` |
| macOS (Intel) | `gguf_analyzer-x86_64-apple-darwin.tar.xz` |
| Linux (x86-64) | `gguf_analyzer-x86_64-unknown-linux-gnu.tar.xz` |
| Linux (ARM64) | `gguf_analyzer-aarch64-unknown-linux-gnu.tar.xz` |
| Windows (64-bit) | `gguf_analyzer-x86_64-pc-windows-msvc.zip` |

Each archive has a sibling `<file>.sha256` for checksum verification.

**macOS / Linux:**

```bash
# Unpack (replace filename with the one you downloaded)
tar -xJf gguf_analyzer-aarch64-apple-darwin.tar.xz
chmod +x gguf-analyzer
# Put it somewhere on your PATH
sudo mv gguf-analyzer /usr/local/bin/
# Verify
gguf-analyzer --version
```

> **macOS Gatekeeper warning?** Run:
> `xattr -d com.apple.quarantine /usr/local/bin/gguf-analyzer`
> or right-click the binary → Open → Open.

**Windows:**

1. Extract the `.zip` file.
2. Move `gguf-analyzer.exe` into a folder on your `PATH` (e.g. `C:\Tools\`).
3. Open a new terminal and run `gguf-analyzer --version`.

---

### Option C — Install from source (requires Rust)

```bash
# Install Rust: https://rustup.rs
git clone https://github.com/mitanuriel/gguf_analyzer
cd gguf_analyzer
cargo install --path .
gguf-analyzer --version
```

---

## Quick start

Download any GGUF model (e.g. from [Hugging Face](https://huggingface.co/models?library=gguf))
and run:

```bash
gguf-analyzer info    model.gguf
gguf-analyzer meta    model.gguf
gguf-analyzer tensors model.gguf
```

---

## Features

- Inspect file header, tensor count, and alignment (`info`)
- Browse and filter metadata with glob patterns (`meta`)
- List all tensors with shapes, quant types, offsets, and sizes (`tensors`)
- Add or change a metadata value without touching the weights (`set`)
- Remove a metadata key (`remove`)
- Export metadata to **JSON**, **Markdown**, or **CSV** (`export`)
- Shell completions for bash, zsh, fish, and PowerShell (`completions`)
- Structured tracing via `RUST_LOG`
- ANSI-coloured output that respects `NO_COLOR`

---

## Commands

### `info` — file summary

```bash
gguf-analyzer info model.gguf
```

---

### `meta` — browse metadata

```bash
# Show all metadata
gguf-analyzer meta model.gguf

# Filter by glob (case-insensitive)
gguf-analyzer meta model.gguf --filter "general.*"

# Limit array preview and filter
gguf-analyzer meta model.gguf --filter "tokenizer.*" --array-limit 5
```

| Flag | Description | Default |
|---|---|---|
| `--filter <GLOB>` | Glob pattern, e.g. `"llama.*"` | show all |
| `--array-limit <N>` | Max array items shown per cell | `8` |

---

### `tensors` — list tensors

```bash
gguf-analyzer tensors model.gguf
gguf-analyzer tensors model.gguf --filter "blk.0.*"
gguf-analyzer tensors model.gguf --filter "*attn_q*"
```

| Flag | Description | Default |
|---|---|---|
| `--filter <GLOB>` | Glob pattern, e.g. `"blk.0.*"` | show all |

---

### `set` — set or add a metadata value

The source file is **never modified**. Always pass `--output`.

```bash
# Preview with --dry-run
gguf-analyzer set model.gguf \
  --key general.name \
  --value "my-custom-model" \
  --type string \
  --output tagged.gguf \
  --dry-run

# Apply
gguf-analyzer set model.gguf \
  --key general.name --value "my-custom-model" \
  --type string --output tagged.gguf

# Add a brand-new key (requires --force)
gguf-analyzer set model.gguf \
  --key custom.build_date --value "2026-05-01" \
  --type string --output tagged.gguf --force
```

| Flag | Description | Required |
|---|---|---|
| `--key <KEY>` | Metadata key to write | ✓ |
| `--value <VALUE>` | New value | ✓ |
| `-t, --type <TYPE>` | `u8` `i8` `u16` `i16` `u32` `i32` `f32` `u64` `i64` `f64` `bool` `string` | ✓ |
| `-o, --output <FILE>` | Output file path | ✓ |
| `--force` | Allow new key or overwrite existing output | — |
| `--backup` | Before overwriting `<output>`, rename existing file to `<output>.bak` | — |
| `--dry-run` | Preview without writing | — |

---

### `remove` — remove a metadata key

```bash
# Preview
gguf-analyzer remove model.gguf \
  --key general.quantization_version \
  --output trimmed.gguf --dry-run

# Apply
gguf-analyzer remove model.gguf \
  --key general.quantization_version --output trimmed.gguf
```

| Flag | Description | Required |
|---|---|---|
| `--key <KEY>` | Key to delete | ✓ |
| `-o, --output <FILE>` | Output file path | ✓ |
| `--force` | Overwrite output if it exists | — |
| `--backup` | Before overwriting `<output>`, rename existing file to `<output>.bak` | — |
| `--dry-run` | Preview without writing | — |

---

### `export` — export metadata

```bash
gguf-analyzer export model.gguf --format json
gguf-analyzer export model.gguf --format markdown --output meta.md
gguf-analyzer export model.gguf --format csv --output meta.csv
```

| Flag | Description | Default |
|---|---|---|
| `-f, --format` | `json` \| `markdown` \| `csv` | `json` |
| `-o, --output <FILE>` | Write to file instead of stdout | stdout |
| `--array-limit <N>` | Max array items per value | `8` |

---

### `completions` — shell completions

```bash
gguf-analyzer completions zsh  > ~/.zfunc/_gguf-analyzer
gguf-analyzer completions bash > /etc/bash_completion.d/gguf-analyzer
gguf-analyzer completions fish > ~/.config/fish/completions/gguf-analyzer.fish
gguf-analyzer completions powershell >> $PROFILE
```

---

## Environment variables

| Variable | Description |
|---|---|
| `RUST_LOG` | Tracing level. `RUST_LOG=debug gguf-analyzer info model.gguf` prints spans to stderr. Values: `error` · `warn` (default) · `info` · `debug` · `trace`. |
| `NO_COLOR` | Set to any value to disable ANSI colour output. |

---

## Releasing a new version

This project uses [cargo-dist](https://github.com/axodotdev/cargo-dist).
Binaries for all five platforms are built automatically by GitHub Actions
whenever a version tag is pushed.

```bash
# 1. Bump version in Cargo.toml, commit
# 2. Tag and push
git tag v0.2.0
git push --tags
```

GitHub will build the binaries and attach them to the Releases page. Done.

---

## Tech stack

| Crate | Purpose |
|---|---|
| `gguf-rs-lib` | GGUF header + metadata parsing and serialisation |
| `memmap2` | Memory-mapped file I/O for zero-copy tensor access |
| `clap 4` | CLI argument parsing (derive API) |
| `clap_complete` | Shell completion generation |
| `tabled` | Terminal-width-aware table rendering with ANSI colour |
| `colored` | ANSI colour in non-table output |
| `tracing` + `tracing-subscriber` | Structured levelled diagnostics |
| `anyhow` + `thiserror` | Error handling |
| `serde_json` | JSON export |
| `glob` | Glob pattern filtering |
| `terminal_size` | Terminal width detection |
| `cargo-dist` | Cross-platform release automation |

---

## License

MIT
