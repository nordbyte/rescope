# record

Measure a bounded duration and print an aggregate report.

```bash
rescope record [OPTIONS]
```

## Examples

```bash
rescope record --duration 30s --interval 1s
rescope record --duration 1m --group user --sort io
rescope record --duration 30s --profile memory --include-idle
rescope record --duration 1m --name node --json report.json
rescope record --duration 30s --raw-samples raw.json
rescope record --duration 5m --include-idle --all
```

## Options

- `--duration <DURATION>` accepts `ms`, `s`, `m`, `h`; default `30s`
- `--interval <DURATION>` sample interval, default `1s`, minimum `250ms`
- `--timeline <N>` number of CPU, I/O and RAM sparkline rows in terminal output
- `--include-idle` include rows with no observed CPU, I/O or RAM movement
- `--raw-samples <PATH>` write replayable raw samples before filtering
- `--all` disables the row limit and includes idle rows
- `--normalize-cpu` display CPU divided by logical CPU count

Common filters, grouping and exports are documented in [CLI options](../reference/options.md).
Recording reports include approximate CPU/RAM/I/O percentiles, process details and started/exited process counts.
Use [replay](replay.md) to rebuild reports from raw samples with different filters or grouping.
