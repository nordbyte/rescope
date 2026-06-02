# CLI options

## Global options

- `--color <auto|always|never>` controls color output.
- `--no-color` disables color output.
- `--json <PATH>` writes JSON. Use `-` for stdout.
- `--csv <PATH>` writes CSV. Use `-` for stdout.
- `--bytes` prints byte counts instead of human-readable units.
- `-v`, `--verbose` reserved for additional diagnostics.
- `-q`, `--quiet` suppresses terminal tables and status lines.

## Filters

- `--pid <PID>`
- `--user <USER>`
- `--name <NAME>`
- `--cmd <SUBSTRING>`
- `--hide-self`
- `--show-command`

## Grouping

```text
--group <process|name|user|command|executable|parent>
```

- `--group <GROUP>` selects the aggregation key.

## Sorting

```text
--sort <cpu|ram|read|write|io|pid|name|user>
```

## Output size and CPU display

- `--limit <N>` limits visible rows.
- `--all` disables truncation. For `record`, it also includes idle rows.
- `--normalize-cpu` displays process CPU as a share of all logical CPUs.

## Durations

Durations accept `ms`, `s`, `m` and `h`, for example `250ms`, `30s`, `5m` or `1h`.
