# Practicum Rust

Rust workspace for practicum assignments.

## Workspace

```text
practicum-rust/
├── crates/ # assignment projects
├── docs/   # Nextra documentation
└── xtask/  # build, test, and documentation automation
```

The root Cargo workspace currently contains only the `xtask` crate. Some assignments, such as
`crates/mini-launchpad`, are kept as standalone nested projects because they have their own
Rust/Node/Solana toolchains and Makefile flow.

## Commands

```bash
cargo xfmt       # format the workspace
cargo xclippy    # run clippy with warnings denied
cargo xtest      # run tests configured by xtask
cargo ci         # fmt-check + clippy + tests
cargo docs       # build documentation
cargo docs-dev   # run the documentation dev server
```
