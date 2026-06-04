# CLI options

## Global options

- `--color <auto|always|never>` controls color output.
- `--no-color` disables color output.
- `--json <PATH>` writes JSON. Use `-` for stdout.
- `--csv <PATH>` writes CSV. Use `-` for stdout.
- `--bytes` prints byte counts instead of human-readable units.
- `-v`, `--verbose` prints sampler configuration, match counts and export diagnostics to stderr.
- `--config <PATH>` loads default options from a JSON config file. CLI flags that use non-default values take practical precedence.
- `--config-profile <NAME>` applies a named overlay from `profiles.NAME` inside the config file.
- `-q`, `--quiet` suppresses terminal tables and status lines.

## Filters

- `--pid <PID>`
- `--user <USER>`
- `--process <SUBSTRING>`
- `--name <NAME>`
- `--name-regex <REGEX>`
- `--cmd <SUBSTRING>`
- `--cmd-regex <REGEX>`
- `--exe <SUBSTRING>`
- `--path <SUBSTRING>` alias for `--exe`
- `--exe-regex <REGEX>`
- `--parent <PID>`
- `--parent-name <SUBSTRING>`
- `--parent-regex <REGEX>`
- `--min-cpu <PERCENT>`
- `--min-ram <SIZE>`
- `--min-io <SIZE>`
- `--invert`
- `--hide-self`
- `--show-command`
- `--show-path`

Size filters accept raw bytes or binary suffixes such as `512MiB`, `1GiB`, `64KiB` and `10M`.
`--process` is a flexible case-insensitive substring search across PID, process name, executable path and command line. Regex filters are case-insensitive and are validated before sampling starts. `--invert` only changes positive filters; with no positive filter it keeps all rows except `--hide-self`.

## Profiles

```text
--profile <cpu|memory|io|commands|users|tree>
```

Profiles select practical defaults for group, sort and command display:

- `--profile <PROFILE>` applies one of the available profiles.
- `cpu` keeps process rows sorted by CPU.
- `memory` keeps process rows sorted by RAM.
- `io` keeps process rows sorted by combined read/write I/O.
- `commands` groups by command line and enables command display.
- `users` groups by user and sorts by RAM.
- `tree` groups by parent process with parent PID and name.

## Grouping

```text
--group <process|name|user|command|executable|parent|cgroup|systemd|container>
```

- `--group <GROUP>` selects the aggregation key. Parent groups show the parent PID plus the parent process name when the platform reports it.
- `cgroup` groups by the full Linux cgroup path when available.
- `systemd` extracts a `.service`, `.scope` or `.slice` unit from the cgroup path.
- `container` extracts common Docker/containerd/Podman IDs from cgroup paths and uses `host` otherwise.

## Sorting

```text
--sort <cpu|cpu-max|cpu-p95|ram|ram-avg|ram-end|read|write|io|io-avg|pid|name|user|started|exited>
```

For snapshot, live, tree and watch, aggregate-only sort modes map to the nearest current-sample value. For recordings, the extra modes select recording aggregates:

- `cpu-max` maximum CPU
- `cpu-p95` p95 CPU
- `ram-avg` average RAM
- `ram-end` RAM at the last sample
- `io-avg` average combined I/O per second
- `started` process identities that appeared during the recording
- `exited` process identities that disappeared before the recording ended

## Output size and CPU display

- `--limit <N>` limits visible rows.
- `--all` disables truncation. For `record`, it also includes idle rows.
- `--normalize-cpu` displays process CPU as a share of all logical CPUs.

## Command-specific options

### snapshot

- `--show-system` prints the system summary.
- `--no-system` hides the system summary.

### live

- `--once` renders one sample and exits.
- `--tui` starts the interactive terminal UI when a TTY is available.
- `--plain` forces plain refresh mode.
- `--jsonl <PATH>` streams newline-delimited JSON snapshots. Use `-` for stdout with `--quiet`.
- `--csv-stream <PATH>` streams snapshot rows as CSV. Use `-` for stdout with `--quiet`.
- `--prometheus <TARGET>` publishes Prometheus text metrics to `-`, a file, or an HTTP bind address such as `127.0.0.1:9898`.

### record

- `--duration <DURATION>` recording duration, default `30s`.
- `--timeline <N>` number of CPU, I/O and RAM timeline rows in terminal output.
- `--include-idle` keeps rows with no observed CPU, I/O or RAM movement.
- `--raw-samples <PATH>` writes replayable raw samples before filtering.

### replay

- `replay <PATH>` reads raw samples written by `record --raw-samples`.
- Replay accepts the common filters, grouping, sorting, `--timeline`, `--include-idle`, `--json` and `--csv`.

### tree

- `--limit <N>` maximum process nodes to print, default `100`.
- `--all` prints every matching node.

### watch

- `--duration <DURATION>` maximum watch duration, default `30s`.
- `--for <DURATION>` requires a match to remain present continuously before alerting.
- `--stream` prints every matching sample until the duration ends.
- `--exit-code <1-255>` exit code when filters match, default `10`.

### diff

- `--limit <N>` maximum changed rows, default `20`.
- `--all` keeps every changed row.

## Durations

Durations accept `ms`, `s`, `m` and `h`, for example `250ms`, `30s`, `5m` or `1h`.

## Config file

The config file is JSON and supports global defaults plus common command defaults:

```json
{
  "profile": "io",
  "limit": 15,
  "interval": "1s",
  "normalize_cpu": true,
  "hide_self": true,
  "bytes": false
}
```

Supported top-level fields are `color`, `no_color`, `bytes`, `quiet`, `profile`, `group`, `sort`, `limit`, `interval`, `normalize_cpu`, `show_command`, `show_path`, `hide_self`, `include_idle`, `pids`, `users`, `process`, `names`, `name_regexes`, `command`, `command_regexes`, `executable`, `executable_regexes`, `parent_pids`, `parent_names`, `parent_regexes`, `min_cpu`, `min_ram`, `min_io`, `invert`, `duration`, `timeline`, `all`, `show_system`, `once`, `tui`, `plain`, `jsonl`, `csv_stream`, `prometheus`, `stream` and `exit_code`.

Command-specific overlays can override those defaults under `snapshot`, `live`, `record`, `tree` and `watch`:

```json
{
  "limit": 20,
  "hide_self": true,
  "live": {
    "tui": true,
    "jsonl": "live.jsonl"
  },
  "watch": {
    "duration": "5m",
    "exit_code": 20,
    "min_cpu": 80
  },
  "profiles": {
    "containers": {
      "group": "container",
      "sort": "cpu"
    }
  }
}
```
