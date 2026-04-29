# Practicum Rust

Rust workspace for practicum assignments.

## Workspace

```text
practicum-rust/
├── docs/   # Nextra documentation
└── xtask/  # build, test, and documentation automation
```

Current assignment crates:

- `crates/broken-app` - fixed module 5 debugging and optimization project.
- `crates/reference-app` - unchanged reference project, kept outside the workspace via `exclude`.

## Commands

```bash
cargo xfmt       # format the workspace
cargo xclippy    # run clippy with warnings denied
cargo xtest      # run tests configured by xtask
cargo ci         # fmt-check + clippy + tests
cargo docs       # build documentation
cargo docs-dev   # run the documentation dev server
```
