# gguf-analyzer

A fast, colourful CLI tool for exploring and editing the metadata of
[GGUF](https://github.com/ggerganov/ggml/blob/master/docs/gguf.md) model files.
Reads files via memory-map so even 50 GB models open instantly.
Writes are always to a **new output file** — the source is never touched.

---

## Features

- Inspect file header, tensor count, and alignment at a glance
- Browse and filter metadata key-value pairs with glob patterns
- List tensors with shapes, quantisation types, offsets, and sizes
- Add or change a metadata value without re-encoding weights
- Remove a metadata key
- Export metadata to **JSON**, **Markdown**, or **CSV**
- Structured tracing via `RUST_LOG` (powered by [`tracing`](https://docs.rs/tracing))
- ANSI-coloured output (bold-cyan headers, dimmed types, green sizes)

---

## Installation

```bash
git clone https://github.com/mitanuriel/gguf_analyzer
cd gguf_analyzer
cargo build --release
# Binary is at ./target/release/gguf-analyzer
```

Add to your PATH if you want to use it anywhere:

```bash
cp target/release/gguf-analyzer ~/.local/bin/
```

---

## Usage

```
gguf-analyzer <COMMAND> <FILE> [OPTIONS]
```

Run `gguf-analyzer --help` or `gguf-analyzer <COMMAND> --help` for the
full option list.

---

## Commands

### `info` — file summary

Print a high-level overview of the file: size, GGUF version, tensor
count, metadata count, alignment, and the byte offset where tensor data
starts.

```bash
gguf-analyzer info model.gguf
```

Example output:

```
╭─────────────────────┬───────────────────────────────────────╮
│ Field               │ Value                                 │
├─────────────────────┼───────────────────────────────────────┤
│ File                │ model.gguf                            │
│ File size           │ 565.05 MiB                            │
│ GGUF version        │ 3                                     │
│ Tensor count        │ 201                                   │
│ Metadata entries    │ 23                                    │
│ Alignment           │ 32 bytes                              │
│ Tensor data offset  │ 0x001a1580  (1.63 MiB)               │
╰─────────────────────┴───────────────────────────────────────╯
```

---

### `meta` — browse metadata

Display all metadata key-value pairs in a table.

```bash
gguf-analyzer meta model.gguf
```

**Options:**

| Flag | Description | Default |
|---|---|---|
| `--filter <GLOB>` | Case-insensitive glob, e.g. `"llama.*"` | show all |
| `--array-limit <N>` | Max array elements shown per cell | `8` |

**Examples:**

```bash
# Show only keys that start with "general."
gguf-analyzer meta model.gguf --filter "general.*"

# Limit array values to 3 items
gguf-analyzer meta model.gguf --array-limit 3

# Combine both
gguf-analyzer meta model.gguf --filter "tokenizer.*" --array-limit 5
```

---

### `tensors` — list tensors

Display all tensors with their shape, quantisation type, data offset,
and size.

```bash
gguf-analyzer tensors model.gguf
```

**Options:**

| Flag | Description | Default |
|---|---|---|
| `--filter <GLOB>` | Case-insensitive glob, e.g. `"blk.0.*"` | show all |

**Examples:**

```bash
# Show all tensors in block 0
gguf-analyzer tensors model.gguf --filter "blk.0.*"

# Show only attention weight tensors
gguf-analyzer tensors model.gguf --filter "*attn_q*"
```

---

### `set` — set or add a metadata value

Write a new GGUF file with one metadata key changed (or added).
The source file is **never modified**.

```bash
gguf-analyzer set <FILE> \
  --key <KEY> \
  --value <VALUE> \
  --type <TYPE> \
  --output <OUTPUT_FILE>
```

**Options:**

| Flag | Description | Required |
|---|---|---|
| `--key <KEY>` | Metadata key to write | ✓ |
| `--value <VALUE>` | New value (parsed according to `--type`) | ✓ |
| `-t, --type <TYPE>` | Value type: `u8` `i8` `u16` `i16` `u32` `i32` `f32` `u64` `i64` `f64` `bool` `string` | ✓ |
| `-o, --output <FILE>` | Destination file path | ✓ |
| `--force` | Allow creating a new key / overwriting output file | no |
| `--dry-run` | Preview the change without writing any bytes | no |

**Examples:**

```bash
# Preview a name change
gguf-analyzer set model.gguf \
  --key general.name \
  --value "my-custom-model" \
  --type string \
  --output tagged.gguf \
  --dry-run

# Apply it
gguf-analyzer set model.gguf \
  --key general.name \
  --value "my-custom-model" \
  --type string \
  --output tagged.gguf

# Add a brand-new key (requires --force)
gguf-analyzer set model.gguf \
  --key custom.build_date \
  --value "2026-05-01" \
  --type string \
  --output tagged.gguf \
  --force
```

---

### `remove` — remove a metadata key

Write a new GGUF file with one metadata key deleted.
The source file is **never modified**.

```bash
gguf-analyzer remove <FILE> \
  --key <KEY> \
  --output <OUTPUT_FILE>
```

**Options:**

| Flag | Description | Required |
|---|---|---|
| `--key <KEY>` | Metadata key to delete | ✓ |
| `-o, --output <FILE>` | Destination file path | ✓ |
| `--force` | Overwrite the output file if it already exists | no |
| `--dry-run` | Preview the deletion without writing any bytes | no |

**Examples:**

```bash
# Preview
gguf-analyzer remove model.gguf \
  --key general.quantization_version \
  --output trimmed.gguf \
  --dry-run

# Apply
gguf-analyzer remove model.gguf \
  --key general.quantization_version \
  --output trimmed.gguf
```

---

### `export` — export metadata

Export all metadata to JSON, Markdown (GFM table), or CSV.
Output goes to stdout by default; redirect with `--output`.

```bash
gguf-analyzer export <FILE> [OPTIONS]
```

**Options:**

| Flag | Description | Default |
|---|---|---|
| `-f, --format <FORMAT>` | `json` \| `markdown` \| `csv` | `json` |
| `-o, --output <FILE>` | Write to file instead of stdout | stdout |
| `--array-limit <N>` | Max array elements per value | `8` |

**Examples:**

```bash
# Pretty-print JSON to stdout
gguf-analyzer export model.gguf --format json

# Save as Markdown with arrays limited to 3 items
gguf-analyzer export model.gguf --format markdown --array-limit 3 --output meta.md

# CSV for spreadsheet import
gguf-analyzer export model.gguf --format csv --output meta.csv
```

---

### `completions` — shell completions

Print a shell completion script to stdout.

```bash
gguf-analyzer completions <SHELL>
```

Supported shells: `bash`, `zsh`, `fish`, `elvish`, `powershell`.

**Examples:**

```bash
# Zsh — add to your .zshrc or a completions directory
gguf-analyzer completions zsh > ~/.zfunc/_gguf-analyzer

# Bash
gguf-analyzer completions bash > /etc/bash_completion.d/gguf-analyzer

# Fish
gguf-analyzer completions fish > ~/.config/fish/completions/gguf-analyzer.fish
```

---

## Environment Variables

| Variable | Description |
|---|---|
| `RUST_LOG` | Tracing level for structured diagnostics. E.g. `RUST_LOG=debug gguf-analyzer info model.gguf` prints parse/write spans to stderr. Accepted values: `error`, `warn` (default), `info`, `debug`, `trace`. |
| `NO_COLOR` | Set to any value to disable ANSI colour output. |

---

## Running Tests

```bash
cargo test
```

49 unit tests covering byte formatting, value parsing, shape formatting,
alignment arithmetic, table building, CLI argument parsing, and
error-handling paths.

---

## Tech Stack

| Crate | Purpose |
|---|---|
| `gguf-rs-lib` | GGUF header + metadata parsing and serialisation |
| `memmap2` | Memory-mapped file I/O for zero-copy tensor access |
| `clap 4` | CLI argument parsing (derive API) |
| `clap_complete` | Shell completion generation |
| `tabled` | Terminal-width-aware table rendering |
| `colored` | ANSI colour output |
| `tracing` + `tracing-subscriber` | Structured, levelled diagnostics |
| `anyhow` + `thiserror` | Error handling |
| `serde_json` | JSON export |
| `glob` | Glob pattern filtering |
| `terminal_size` | Terminal width detection |

---

## License

MIT
