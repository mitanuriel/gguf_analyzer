# gguf-analyzer

[![CI](https://github.com/mitanuriel/gguf_analyzer/actions/workflows/ci.yml/badge.svg)](https://github.com/mitanuriel/gguf_analyzer/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/mitanuriel/gguf_analyzer?include_prereleases&sort=semver)](https://github.com/mitanuriel/gguf_analyzer/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/mitanuriel/gguf_analyzer/total)](https://github.com/mitanuriel/gguf_analyzer/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust 2024](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)

A fast CLI tool for exploring and editing the metadata of
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

### `model card from HuggingFace` — model data and recommended sampling parameters
<img width="403" height="273" alt="Screenshot 2026-05-02 at 07 24 33" src="https://github.com/user-attachments/assets/d393da92-4355-4847-b81b-dc52f99482b4" />

<img width="642" height="655" alt="Screenshot 2026-05-02 at 07 26 33" src="https://github.com/user-attachments/assets/9607956b-b3cf-49e2-9961-eb09f92178af" />






### `apply-sampling` — bake recommended sampling params into a GGUF file
<img width="666" height="461" alt="Screenshot 2026-05-02 at 10 04 13" src="https://github.com/user-attachments/assets/38da08bc-2ed5-491e-883a-970f28689135" />


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

---

## Quick start

```bash
# cd to your models folder — then you only need the filename
cd ~/models/my-model

gguf-analyzer info    model.gguf
gguf-analyzer meta    model.gguf
gguf-analyzer tensors model.gguf
```

Or download a model from Hugging Face first (replace `owner/repo` with the actual repo):

```bash
gguf-analyzer fetch owner/repo --file "*Q4_K_M*" --output-dir ~/models/my-model
```

---

## Features

- Inspect file header, tensor count, and alignment (`info`)
- Browse and filter metadata with glob patterns (`meta`)
- List all tensors with shapes, quant types, offsets, and sizes (`tensors`)
- Add or change a metadata value without touching the weights (`set`)
- Remove a metadata key (`remove`)
- Export metadata to **JSON**, **Markdown**, or **CSV** (`export`)
- Download GGUF files from Hugging Face (`fetch`)
- View model card and recommended sampling parameters (`model-card`)
- Bake sampling parameters directly into a GGUF file (`apply-sampling`)
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

# Substring match — wrap in * on both sides
gguf-analyzer meta model.gguf --filter "*attn*"

# Limit array preview
gguf-analyzer meta model.gguf --filter "tokenizer.*" --array-limit 5
```

| Flag | Description | Default |
|---|---|---|
| `--filter <GLOB>` | Glob pattern. Use `*keyword*` for substring match, `general.*` for prefix. | show all |
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
| `--filter <GLOB>` | Glob pattern. Use `*keyword*` for substring match. | show all |

---

### `set` — set or add a metadata value

The source file is **never modified**. Output defaults to `<stem>-modified.gguf` in the same folder as the source.

```bash
# Preview first with --dry-run (no --output needed)
gguf-analyzer set model.gguf \
  --key general.name --value "my-custom-model" --type string --dry-run

# Apply — output auto-named model-modified.gguf
gguf-analyzer set model.gguf \
  --key general.name --value "my-custom-model" --type string

# Explicit output path
gguf-analyzer set model.gguf \
  --key general.name --value "my-custom-model" --type string \
  --output tagged.gguf

# Add a brand-new key (requires --force)
gguf-analyzer set model.gguf \
  --key custom.build_date --value "2026-05-01" --type string --force
```

| Flag | Description | Required |
|---|---|---|
| `--key <KEY>` | Metadata key to write | ✓ |
| `--value <VALUE>` | New value | ✓ |
| `-t, --type <TYPE>` | `u8` `i8` `u16` `i16` `u32` `i32` `f32` `u64` `i64` `f64` `bool` `string` | ✓ |
| `-o, --output <FILE>` | Output path — defaults to `<stem>-modified.gguf` | — |
| `--force` | Allow new key or overwrite existing output | — |
| `--backup` | Rename existing output to `<output>.bak` before writing | — |
| `--dry-run` | Preview without writing | — |

---

### `remove` — remove a metadata key

The source file is **never modified**. Output defaults to `<stem>-modified.gguf`.

```bash
# Preview
gguf-analyzer remove model.gguf --key general.quantization_version --dry-run

# Apply — output auto-named model-modified.gguf
gguf-analyzer remove model.gguf --key general.quantization_version

# Explicit output path
gguf-analyzer remove model.gguf \
  --key general.quantization_version --output trimmed.gguf
```

| Flag | Description | Required |
|---|---|---|
| `--key <KEY>` | Key to delete | ✓ |
| `-o, --output <FILE>` | Output path — defaults to `<stem>-modified.gguf` | — |
| `--force` | Overwrite output if it exists | — |
| `--backup` | Rename existing output to `<output>.bak` before writing | — |
| `--dry-run` | Preview without writing | — |

---

### `export` — export metadata

```bash
# JSON to stdout
gguf-analyzer export model.gguf --format json

# Markdown to file
gguf-analyzer export model.gguf --format markdown --output meta.md

# CSV to file
gguf-analyzer export model.gguf --format csv --output meta.csv
```

| Flag | Description | Default |
|---|---|---|
| `-f, --format` | `json` \| `markdown` \| `csv` | `json` |
| `-o, --output <FILE>` | Write to file instead of stdout | stdout |
| `--array-limit <N>` | Max array items per value | `8` |

---

### `fetch` — download from Hugging Face

```bash
# Interactive file picker
gguf-analyzer fetch Qwen/Qwen3-0.6B-GGUF

# List available files without downloading
gguf-analyzer fetch Qwen/Qwen3-0.6B-GGUF --list

# Download a specific quantisation, save to a folder
gguf-analyzer fetch Qwen/Qwen3-0.6B-GGUF --file "*Q4_K_M*" --output-dir ~/models

# Direct URL
gguf-analyzer fetch https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/resolve/main/Qwen3-0.6B-Q8_0.gguf
```

| Flag | Description | Default |
|---|---|---|
| `-l, --list` | List available files, don't download | — |
| `-f, --file <PATTERN>` | Glob pattern to select file, e.g. `*Q4_K_M*` | interactive prompt |
| `-o, --output-dir <DIR>` | Directory to save into (created automatically if missing) | current directory |
| `--force` | Overwrite if file already exists | — |

---

### `model-card` — view model card from Hugging Face

Fetches the model's README and parses the overview, architecture details, and any recommended sampling parameters.

```bash
gguf-analyzer model-card owner/repo

# Machine-readable JSON output
gguf-analyzer model-card owner/repo --json
```

| Flag | Description |
|---|---|
| `--json` | Output as JSON instead of a human-readable table |

---

### `apply-sampling` — bake sampling parameters into a GGUF file

Reads recommended sampling parameters from a model card and writes them as metadata keys into a new GGUF file. Falls back to an interactive prompt if no recommendations are found.

The source file is **never modified**. Output defaults to `<stem>-sampled.gguf`.

```bash
# Pull params from model card, interactive set picker
gguf-analyzer apply-sampling model.gguf --repo owner/repo

# Select a specific mode (e.g. thinking / non-thinking)
gguf-analyzer apply-sampling model.gguf --repo owner/repo --mode thinking

# Preview without writing
gguf-analyzer apply-sampling model.gguf --repo owner/repo --dry-run

# Enter all parameters manually (no repo needed)
gguf-analyzer apply-sampling model.gguf

# Explicit output path
gguf-analyzer apply-sampling model.gguf --repo owner/repo \
  --output model-thinking.gguf
```

| Flag | Description | Default |
|---|---|---|
| `--repo <REPO>` | Hugging Face repo to fetch params from (`owner/repo` or full URL) | interactive prompts |
| `--mode <LABEL>` | Which sampling set to apply when multiple are available (e.g. `thinking`, `non-thinking`) | first set found |
| `-o, --output <FILE>` | Output path — defaults to `<stem>-sampled.gguf` | — |
| `--force` | Overwrite output if it already exists | — |
| `--backup` | Rename existing output to `<output>.bak` before writing | — |
| `--dry-run` | Show what would be written without writing | — |

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
| `RUST_LOG` | Tracing level — `error` · `warn` (default) · `info` · `debug` · `trace`. Example: `RUST_LOG=debug gguf-analyzer info model.gguf` |
| `NO_COLOR` | Set to any value to disable ANSI colour output. |

---

## Releasing a new version

This project uses [cargo-dist](https://github.com/axodotdev/cargo-dist).
Binaries for all five platforms are built automatically by GitHub Actions whenever a version tag is pushed.

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
| `reqwest` | Hugging Face API and file downloads |
| `indicatif` | Download progress bar |
| `glob` | Glob pattern filtering |
| `terminal_size` | Terminal width detection |
| `cargo-dist` | Cross-platform release automation |

---

## License

MIT
