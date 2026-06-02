# record

Measure a bounded duration and print an aggregate report.

```bash
rescope record --duration <DURATION> [OPTIONS]
```

## Examples

```bash
rescope record --duration 30s --interval 1s
rescope record --duration 1m --group user --sort io
rescope record --duration 1m --name node --json report.json
rescope record --duration 5m --include-idle --all
```

## Options

- `--duration <DURATION>` required, accepts `ms`, `s`, `m`, `h`
- `--interval <DURATION>` sample interval, default `1s`, minimum `250ms`
- `--timeline <N>` number of RAM sparkline rows in terminal output
- `--include-idle` include rows with no observed CPU, I/O or RAM movement
- `--all` disables the row limit and includes idle rows
- `--normalize-cpu` display CPU divided by logical CPU count

Common filters, grouping and exports are documented in [CLI options](../reference/options.md).
