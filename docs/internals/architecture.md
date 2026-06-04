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
- system network deltas
- process identity
- filtering
- grouping
- streaming and batch aggregation
- sorting
- report data structures
- unit formatting helpers

## rescope-cli

The CLI crate owns:

- Clap argument parsing
- JSON config default merging
- command execution
- plain and interactive terminal output
- JSON, JSONL and CSV export
- raw sample replay and Prometheus export
- live loop and streaming recording loop control

## npm/rescope

The npm package is a wrapper. It locates a native binary in an optional platform package, a vendor directory or a local Cargo target directory. Platform package metadata lives under `npm/native`.
