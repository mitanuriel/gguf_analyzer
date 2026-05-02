# Contributing to gguf-analyzer

Thanks for your interest! This is a small project, so the workflow is simple.

## Quick start

```bash
git clone https://github.com/mitanuriel/gguf_analyzer
cd gguf_analyzer
cargo test          # run the full test suite
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

## Reporting bugs

Open a [GitHub Issue](https://github.com/mitanuriel/gguf_analyzer/issues) and include:

1. The exact command you ran
2. The full output (with `RUST_LOG=debug` if relevant)
3. Your platform (`uname -a` on macOS/Linux, `ver` on Windows)
4. The output of `gguf-analyzer info <file>` if a specific GGUF triggers it

If the bug involves a particular GGUF file you cannot share, a synthetic
fixture that reproduces the issue is just as good — see `tests/common.rs` for
how to build one.

## Proposing a feature

Open an [Issue](https://github.com/mitanuriel/gguf_analyzer/issues) or a
[Discussion](https://github.com/mitanuriel/gguf_analyzer/discussions) first
to scope it before opening a PR. Avoids wasted work.

## Pull requests

- Fork the repo and create a topic branch off `master`.
- Keep PRs focused — one logical change per PR.
- Add or update tests. The bar is "no PR lowers test coverage".
- Run locally before pushing:
  ```bash
  cargo test --all-targets
  cargo clippy --all-targets -- -D warnings
  cargo fmt --all -- --check
  ```
- All three must pass on CI before a PR can be merged.
- PRs are merged by **squash** to keep history linear. Write a meaningful
  PR title — it becomes the commit message.

## Project layout

```
src/
  cli.rs            # clap argument definitions
  commands/         # one module per subcommand
  display.rs        # table/colour formatting helpers
  error.rs          # AppError enum (thiserror)
  gguf.rs           # ParsedGguf, write_modified_gguf, helpers
  lib.rs            # re-exports for integration tests
  main.rs           # binary entry point
tests/
  common.rs         # synthetic GGUF fixture
  *_tests.rs        # one file per subcommand
```

## Conduct

By participating you agree to abide by the
[Code of Conduct](CODE_OF_CONDUCT.md).
