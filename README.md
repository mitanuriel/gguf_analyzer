# gguf-analyzer

A fast, colourful CLI tool for exploring and editing the metadata of
[GGUF](https://github.com/ggerganov/ggml/blob/master/docs/gguf.md) model files —
the format used by llama.cpp, Ollama, LM Studio, and most local AI tools.

Reads files via **memory-map** so even 50 GB models open instantly.
Writes are always to a **new output file** — the source is never touched.

---

## Screenshots

### `info` — file overview

![info command output showing file size, GGUF version, tensor count and metadata count](docs/screenshots/info.png)

### `meta` — metadata table

![meta command showing key, type, and value columns with colour-coded rows](docs/screenshots/meta.png)

### `tensors` — tensor inventory

![tensors command with name, shape, quantisation type, offset, and size columns](docs/screenshots/tensors.png)

---

## Installation

### Option A — Download a pre-built binary (no Rust required)

Go to the [**Releases page**](https://github.com/mitanuriel/gguf_analyzer/releases) and
download the archive for your platform:

| Platform | File to download |
|---|---|
| macOS (Apple Silicon M1/M2/M3) | `gguf-analyzer-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `gguf-analyzer-x86_64-apple-darwin.tar.gz` |
| Linux (x86-64) | `gguf-analyzer-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (ARM64) | `gguf-analyzer-aarch64-unknown-linux-gnu.tar.gz` |
| Windows (64-bit) | `gguf-analyzer-x86_64-pc-windows-msvc.zip` |

**macOS / Linux:**

```bash
# Unpack (replace filename with the one you downloaded)
tar -xzf gguf-analyzer-aarch64-apple-darwin.tar.gz
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

### Option B — Install from source (requires Rust)

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
