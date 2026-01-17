# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
cargo ci            # Full CI: format check + clippy + tests for whole workspace
cargo test          # Run all workspace tests
cargo docs          # Build full workspace documentation (rustdoc JSON → Markdown → Nextra)
cargo docs-dev      # Run docs dev server on localhost:3000
cargo xfmt          # Format code with cargo-fmt for whole workspace
cargo xclippy       # Lint with clippy (warnings as errors) for whole workspace
```

General commands above are aliases defined in `.cargo/config.toml` that invoke the `xtask` crate.

## Project Architecture

This is a Rust workspace with 4 crates designed for transaction format processing:

```
crates/parser/     - Core parsing library for transaction formats
tools/converter/   - CLI tool for format conversion
tools/comparer/    - CLI tool for file comparison
xtask/             - Build automation (CI, docs generation)
docs/              - Nextra/Next.js documentation site
```

**Transaction formats** (documented in `docs/content/transactions/`):

- YPBankText - Text format
- YPBankBin - Binary format
- YPBankCsv - CSV format

## Toolchain

- Rust edition 2024 with nightly toolchain (required for rustdoc JSON generation, some Clippy lints)
- Code formatting: `rustfmt.toml` with 100-char line width
- Documentation: Auto-generated from rustdoc JSON using `rustdoc-md`, served via Nextra as static site on GitHub Pages.

## CI Pipeline

GitHub Actions runs `cargo ci` which executes:

1. `cargo fmt --all -- --check` - Verify formatting
2. `cargo clippy --workspace -- -D warnings` - Lint with warnings as errors
3. `cargo test --workspace` - Run all tests
