# Quickstart

## One-shot snapshot

```bash
rescope snapshot --limit 10
```

Group by user and sort by resident memory:

```bash
rescope snapshot --group user --sort ram --limit 10
```

Use a profile when you do not want to remember group and sort flags:

```bash
rescope snapshot --profile io --limit 10
rescope snapshot --profile tree --parent-name systemd
```

## Live mode

Running `rescope` without a subcommand opens the interactive live TUI when a terminal is available. Plain refresh mode is still available explicitly:

```bash
rescope live --interval 1s --limit 20
```

Interactive terminal mode:

```bash
rescope live --tui --group command --sort cpu
```

Press `o` for options, `s` for sort, `g` for grouping, `f` for filters, `v` for view settings, `r` for recording, `e` for export, `?` for help, `/` for search, `Enter` for row details and `q`, `Esc` or `Ctrl-C` to exit. In details, press `f` to switch between frozen details and following the same process or group identity.

## Record a window

```bash
rescope record --duration 30s --interval 1s --limit 10
```

Memory-focused recording:

```bash
rescope record --duration 30s --profile memory --include-idle
```

Filter to one process family:

```bash
rescope record --duration 1m --name node --group process
```

Search flexibly across PID, process name, executable path and command line, and show the executable path:

```bash
rescope snapshot --process node --show-path
```

Export machine-readable output:

```bash
rescope record --duration 1m --group user --json report.json --csv report.csv
```

Stream JSON to another program:

```bash
rescope live --once --json - | jq '.rows[0]'
```

Stream continuous live data:

```bash
rescope live --quiet --jsonl live.jsonl
rescope live --quiet --csv-stream live.csv
rescope live --prometheus 127.0.0.1:9898
```

Inspect process ancestry and subtree totals:

```bash
rescope tree --process node --show-path
```

Watch for a threshold and compare reports:

```bash
rescope watch --name postgres --min-cpu 80 --for 30s --duration 5m
rescope diff before.json after.json
```

Store raw samples for later replay:

```bash
rescope record --duration 30s --raw-samples raw.json
rescope replay raw.json --group container --sort cpu-max
```
