# Practicum Rust

Rust workspace for practicum assignments.

## Workspace

```text
practicum-rust/
├── docs/   # Nextra documentation
└── xtask/  # build, test, and documentation automation
```

At the moment the workspace contains only the `xtask` crate. New assignment crates can be added
later and registered in the root `Cargo.toml`.

## Commands

```bash
cargo xfmt       # format the workspace
cargo xclippy    # run clippy with warnings denied
cargo xtest      # run tests configured by xtask
cargo ci         # fmt-check + clippy + tests
cargo docs       # build documentation
cargo docs-dev   # run the documentation dev server
```
