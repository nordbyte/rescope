# rescope

[![CI](https://img.shields.io/github/actions/workflow/status/nordbyte/rescope/ci.yml?branch=main&style=flat-square)](https://github.com/nordbyte/rescope/actions/workflows/ci.yml) [![Docs](https://img.shields.io/github/actions/workflow/status/nordbyte/rescope/pages.yml?branch=main&label=docs&style=flat-square)](https://github.com/nordbyte/rescope/actions/workflows/pages.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-ffd60a?style=flat-square)](LICENSE) [![Rust](https://img.shields.io/badge/Rust-2024-b7410e?logo=rust&logoColor=white&style=flat-square)](Cargo.toml) [![npm](https://img.shields.io/badge/npm-rescope-cb3837?logo=npm&logoColor=white&style=flat-square)](npm/rescope/package.json)

Inspect and record resource usage by process and user.

Use the README for the first install and quick start. Full documentation is available in [docs/](docs/index.md) and is deployed with GitHub Pages.

## What rescope shows

- CPU usage per process and aggregated by user, name, command, executable or parent PID.
- Resident memory per process and aggregate RAM start/end/min/max/average/delta during recordings.
- Per-process read and write counters with safe deltas.
- Live views in plain refresh mode or interactive terminal mode.
- Time-bounded recording reports from the CLI or directly from the interactive TUI.
- TUI menus for sorting, grouping, filters, column visibility, sampling, recording and exports.
- JSON and CSV exports to files or stdout.

## Install

From source:

```bash
git clone https://github.com/nordbyte/rescope.git
cd rescope
cargo build -p rescope-cli --release
./target/release/rescope --help
```

Local development:

```bash
cargo run -p rescope-cli -- snapshot --limit 10
```

npm wrapper smoke test:

```bash
cargo build -p rescope-cli
cd npm/rescope
node bin/rescope.js snapshot --limit 10
```

Published install commands, once packages are released:

```bash
cargo install rescope
npm install -g rescope
```

## Quick start

```bash
rescope snapshot --limit 10
rescope snapshot --group user --sort ram --limit 10
rescope snapshot --group executable --sort io --all
rescope snapshot --name-regex '^(node|bun)$' --min-ram 512MiB
rescope live --tui --group command --sort cpu
rescope live --once --json -
rescope record --duration 1m --interval 1s --group user
rescope record --duration 30s --name node --json report.json --csv report.csv
```

Running `rescope` without a subcommand is equivalent to `rescope live`.
In TUI mode, press `o` for the central options menu, `?` for help, `/` for live search, `Enter` for row details, `s` for sort, `v` for columns, `r` for recording and `e` for export. Menus use up/down plus Enter, so grouping, sorting, filters, view options, sampling and exports can be changed without remembering CLI flags.

## Privacy

Command lines are hidden by default because they can contain secrets. Use `--show-command` to display them in process rows. `--cmd` filters command lines internally without changing the default display. `--group command` intentionally displays command lines because command aggregation is explicitly requested.

`rescope` is read-only, requires no root privileges, performs no network requests and sends no telemetry.

## Metrics

CPU values can exceed `100%` on multi-core systems. Use `--normalize-cpu` to display CPU as a share of all logical CPUs. Recording reports calculate CPU core-seconds from the actual elapsed sample interval.

RAM is resident memory when the platform exposes it that way. Disk I/O is platform-dependent: cached operations may not increase counters on Unix-like systems, and Windows counters may include non-disk I/O depending on the OS API.

Recording reports hide rows with no CPU, I/O or RAM movement by default. Use `--include-idle` to keep the current limit and include them, or `--all` to include every row.

## Documentation

| Topic | Link |
| --- | --- |
| Installation | [docs/start/install.md](docs/start/install.md) |
| Quickstart | [docs/start/quickstart.md](docs/start/quickstart.md) |
| Live monitoring | [docs/guides/live.md](docs/guides/live.md) |
| Recording reports | [docs/guides/recording.md](docs/guides/recording.md) |
| Filters and grouping | [docs/guides/filters-grouping.md](docs/guides/filters-grouping.md) |
| Exports | [docs/guides/exports.md](docs/guides/exports.md) |
| CLI command reference | [docs/commands/index.md](docs/commands/index.md) |
| Metrics reference | [docs/reference/metrics.md](docs/reference/metrics.md) |
| Architecture | [docs/internals/architecture.md](docs/internals/architecture.md) |

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run docs:verify
npm run npm:smoke
```

Build the documentation:

```bash
npm run docs:build
```
