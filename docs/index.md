# rescope

Inspect and record resource usage by process and user.

`rescope` is a portable Rust CLI for answering one practical question: which processes, commands, users, cgroups or containers are consuming CPU, RAM and I/O right now, and what changed during a measured window?

## What it does

- Takes one-shot snapshots for scripts and CI.
- Runs a live terminal view with plain refresh or interactive TUI mode.
- Records a bounded time window and prints an aggregate report, including direct analysis after TUI recordings.
- Renders process trees with subtree CPU, RAM and I/O totals.
- Watches for threshold/filter matches and exits with an alert code.
- Diffs two JSON reports to rank before/after changes.
- Groups by process, process name, user, command line, executable path or parent PID.
- Filters by PID, user, flexible process search, process name, command substring and executable path.
- Exports JSON, JSONL and CSV to files or stdout.

## Common commands

```bash
rescope snapshot --limit 10
rescope snapshot --group user --sort ram
rescope live --tui --group command
rescope record --duration 1m --interval 1s --group user
rescope record --duration 30s --raw-samples raw.json
rescope replay raw.json --group container
rescope record --duration 30s --name node --json report.json
rescope tree --process postgres --show-path
rescope watch --name postgres --min-cpu 80 --for 30s --duration 5m
rescope diff before.json after.json
```

## Documentation map

- [Installation](start/install.md)
- [Quickstart](start/quickstart.md)
- [Core concepts](start/core-concepts.md)
- [Live monitoring](guides/live.md)
- [Recording reports](guides/recording.md)
- [Filters and grouping](guides/filters-grouping.md)
- [CLI command reference](commands/index.md)
- [CLI options](reference/options.md)
- [Metrics reference](reference/metrics.md)
- [Architecture](internals/architecture.md)
