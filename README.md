# Practicum Rust

Rust workspace for practicum assignments.

## Workspace

```text
practicum-rust/
├── crates/    # assignment crates
│   └── analysis/
├── docs/      # Nextra documentation
└── xtask/     # build, test, and documentation automation
```

Workspace crates are registered in the root `Cargo.toml`. New assignments should be added under
`crates/`.

## Commands

```bash
cargo xfmt       # format the workspace
cargo xclippy    # run clippy with warnings denied
cargo xtest      # run tests configured by xtask
cargo ci         # fmt-check + clippy + tests
cargo docs       # build documentation
cargo docs-dev   # run the documentation dev server
```
