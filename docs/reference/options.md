# CLI options

## Global options

- `--color <auto|always|never>` controls color output.
- `--no-color` disables color output.
- `--json <PATH>` writes JSON. Use `-` for stdout.
- `--csv <PATH>` writes CSV. Use `-` for stdout.
- `--bytes` prints byte counts instead of human-readable units.
- `-v`, `--verbose` prints sampler configuration, match counts and export diagnostics to stderr.
- `--config <PATH>` loads default options from a JSON config file. CLI flags that use non-default values take practical precedence.
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
--group <process|name|user|command|executable|parent>
```

- `--group <GROUP>` selects the aggregation key. Parent groups show the parent PID plus the parent process name when the platform reports it.

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

Supported fields are `color`, `no_color`, `bytes`, `quiet`, `profile`, `group`, `sort`, `limit`, `interval`, `normalize_cpu`, `show_command`, `show_path`, `hide_self` and `include_idle`.
