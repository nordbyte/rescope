# Architecture

`rescope` is a Rust workspace.

```text
crates/rescope-core
crates/rescope-cli
npm/rescope
docs
```

## rescope-core

The core crate has no terminal UI code. It owns:

- system sampling
- process identity
- filtering
- grouping
- aggregation
- sorting
- report data structures
- unit formatting helpers

## rescope-cli

The CLI crate owns:

- Clap argument parsing
- command execution
- plain and interactive terminal output
- JSON and CSV export
- live loop and recording loop control

## npm/rescope

The npm package is a wrapper. It locates a native binary in an optional platform package, a vendor directory or a local Cargo target directory.
