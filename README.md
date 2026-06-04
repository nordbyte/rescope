# rescope

[![CI](https://img.shields.io/github/actions/workflow/status/nordbyte/rescope/ci.yml?branch=main&style=flat-square)](https://github.com/nordbyte/rescope/actions/workflows/ci.yml) [![Docs](https://img.shields.io/github/actions/workflow/status/nordbyte/rescope/pages.yml?branch=main&label=docs&style=flat-square)](https://github.com/nordbyte/rescope/actions/workflows/pages.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-ffd60a?style=flat-square)](LICENSE) [![Rust](https://img.shields.io/badge/Rust-2024-b7410e?logo=rust&logoColor=white&style=flat-square)](Cargo.toml) [![npm](https://img.shields.io/badge/npm-rescope-cb3837?logo=npm&logoColor=white&style=flat-square)](npm/rescope/package.json)

Inspect and record resource usage by process and user.

Use the README for the first install and quick start. Full documentation is available in [docs/](docs/index.md) and is deployed with GitHub Pages.

## What rescope shows

- CPU usage per process and aggregated by user, name, command, executable, parent PID/name, cgroup, systemd unit or container.
- Flexible process filtering across PID, process name, executable path and command line.
- Resident memory per process and aggregate RAM start/end/min/max/p95/average/delta during recordings.
- Per-process read and write counters with safe deltas.
- Approximate recording percentiles for CPU, RAM and combined I/O plus started/exited process counts.
- Live views in plain refresh mode or interactive terminal mode.
- Time-bounded recording reports from the CLI or directly from the interactive TUI, with immediate TUI analysis after recording.
- Parent-child process trees with subtree CPU, RAM and I/O totals.
- Alert-style watch mode and JSON report diffs for before/after comparisons.
- TUI menus for sorting, grouping, filters, column visibility, sampling, recording, recording analysis, exports and frozen/following details.
- JSON, JSONL, CSV, raw replay and Prometheus exports.

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
rescope snapshot --group container --sort cpu
rescope snapshot --process postgres --show-path
rescope snapshot --path /usr/bin --show-path
rescope snapshot --name-regex '^(node|bun)$' --min-ram 512MiB
rescope snapshot --profile tree --parent-name systemd
rescope live --tui --group command --sort cpu
rescope live --tui --profile io
rescope live --once --json -
rescope live --quiet --jsonl live.jsonl
rescope live --prometheus 127.0.0.1:9898
rescope record --duration 1m --interval 1s --group user
rescope record --duration 30s --profile memory --include-idle
rescope record --duration 30s --name node --json report.json --csv report.csv
rescope record --duration 30s --raw-samples raw.json
rescope replay raw.json --group systemd --sort cpu-max
rescope tree --process postgres --show-path
rescope watch --name postgres --min-cpu 80 --for 30s --duration 5m
rescope diff before.json after.json
rescope completions bash > rescope.bash
rescope man > rescope.1
```

Running `rescope` without a subcommand starts the interactive live TUI when a terminal is available and falls back to plain live output otherwise.
In TUI mode, press `o` for the central options menu, `?` for help, `/` for live search, `Enter` for row details, `s` for sort, `g` for grouping, `f` for filters, `v` for columns, `r` for recording, `a` for the last recording analysis and `e` for export. When a TUI recording finishes, rescope switches from live view into the recording analysis so the captured data can be sorted, grouped, filtered, searched and exported without leaving the TUI. Press `l` to return to live view. In live details, `f` toggles frozen versus following the same process or group identity. Menus use up/down plus Enter, so grouping, sorting, filters, view options, sampling and exports can be changed without remembering CLI flags.

Profiles are available with `--profile cpu|memory|io|commands|users|tree`. A JSON config file can provide defaults:

```json
{
  "profile": "io",
  "limit": 15,
  "interval": "1s",
  "hide_self": true
}
```

Command-specific config sections are available as `snapshot`, `live`, `record`, `tree` and `watch`. Named overlays can be stored under `profiles` and selected with `--config-profile <NAME>`.

Use it with:

```bash
rescope --config rescope.json live --tui
```

## Privacy

Command lines are hidden by default because they can contain secrets. Use `--show-command` to display them in process rows. `--cmd` and `--process` can filter command lines internally without changing the default display. `--group command` intentionally displays command lines because command aggregation is explicitly requested.

Executable paths can include local usernames or project paths. They are hidden by default in process rows; use `--show-path` to display the full executable path. `--path` is an alias for the existing executable-path filter `--exe`.

`rescope` is read-only, requires no root privileges, performs no network requests and sends no telemetry.

## Metrics

CPU values can exceed `100%` on multi-core systems. Use `--normalize-cpu` to display CPU as a share of all logical CPUs. Recording reports calculate CPU core-seconds from the actual elapsed sample interval.

RAM is resident memory when the platform exposes it that way. Disk I/O is platform-dependent: cached operations may not increase counters on Unix-like systems, and Windows counters may include non-disk I/O depending on the OS API.

Recording reports are aggregated as samples arrive and hide rows with no CPU, I/O or RAM movement by default. Use `--include-idle` to keep the current limit and include them, or `--all` to include every row.

Process details such as status, runtime, accumulated CPU time, thread count, open file count and Linux cgroup path are included when the platform exposes them. These Linux `/proc` details are cached briefly per process identity to avoid rereading them on every fast live sample.

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
npm run npm:test
npm run docs:verify
npm run npm:smoke
```

Build the documentation:

```bash
npm run docs:build
```
