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

Plain refresh mode:

```bash
rescope live --interval 1s --limit 20
```

Interactive terminal mode:

```bash
rescope live --tui --group command --sort cpu
```

Press `o` for options, `?` for help, `/` for search, `Enter` for row details and `q`, `Esc` or `Ctrl-C` to exit. In details, press `f` to switch between frozen details and following the same process or group identity.

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

Export machine-readable output:

```bash
rescope record --duration 1m --group user --json report.json --csv report.csv
```

Stream JSON to another program:

```bash
rescope live --once --json - | jq '.rows[0]'
```
