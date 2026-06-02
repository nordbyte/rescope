# rescope

Inspect and record resource usage by process and user.

`rescope` is a Rust CLI for live snapshots and time-bounded reports of CPU, RAM and per-process I/O usage. It groups by process, process name or user, and can export JSON and CSV.

## Installation

```bash
cargo install rescope
npm install -g rescope
```

The npm package is a wrapper around the native Rust binary. It does not implement metric collection in JavaScript.

## Usage

```bash
rescope
rescope snapshot
rescope snapshot --group user --sort ram --limit 10
rescope live --group user
rescope live --sort io --interval 2s --limit 30
rescope record --duration 1m
rescope record --duration 1m --user postgres
rescope record --duration 1m --name node --json report.json
rescope record --duration 30s --pid 1234 --csv report.csv
```

`rescope` without a subcommand is equivalent to `rescope live`.

## Metrics

CPU values are sampled through `sysinfo` and require a warm-up refresh. Process CPU can exceed `100%` on multi-core systems. Recording reports compute CPU core-seconds as `(cpu_percent / 100) * interval_seconds`.

RAM is reported as resident memory when the platform exposes it that way. Recording reports track RAM start, end, min, max, average, delta and a compact terminal sparkline.

Per-process reads and writes are calculated from total I/O counters using safe deltas. The first time a process identity appears, its read/write deltas are `0` so old activity is not counted as part of the recording.

## Platform Notes

Linux x86_64 is the MVP priority, followed by macOS x86_64/aarch64 and Windows x86_64. If a platform cannot provide a metric, `rescope` falls back to `unknown`, `n/a` or `0` instead of crashing.

On Windows, per-process I/O counters may include non-disk I/O depending on the OS counters. On Unix-like systems, cached file operations may not always increase disk counters.

## Privacy

Command lines are hidden by default because they can contain secrets. Use `--show-command` to display them. `--cmd` filters command lines internally without changing the default report display.

`rescope` is read-only, requires no root privileges, performs no network requests and sends no telemetry.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p rescope-cli -- snapshot
cargo run -p rescope-cli -- record --duration 5s --interval 1s --limit 10
```

Interactive TUI mode is planned; current live mode uses plain terminal refresh. The optional `tui` feature keeps the crate layout ready for a later `ratatui` implementation.
