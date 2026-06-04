# live

Render repeated snapshots.

```bash
rescope live [OPTIONS]
```

Running `rescope` without a subcommand starts this live mode in interactive TUI mode when a terminal is available.

## Examples

```bash
rescope live
rescope live --plain --interval 2s
rescope live --tui --group command --sort cpu
rescope live --tui --profile io
rescope --config rescope.json live --tui
rescope live --once --json -
rescope live --quiet --jsonl live.jsonl
rescope live --quiet --csv-stream live.csv
rescope live --prometheus 127.0.0.1:9898
```

## Modes

- Plain mode clears and redraws the terminal, including terminal scrollback.
- TUI mode uses an alternate screen with a central `o` options menu, direct menus for sorting/grouping/filtering/view/recording/export, frozen or following row details, live search, pause/resume, row-limit, interval and column controls. The footer shows the same direct option shortcuts in every main live view.
- `--once` renders one sample and exits.

`--json` and `--csv` are supported only with `--once`. Continuous exports use `--jsonl` or `--csv-stream`; use `--quiet` when a stream writes to stdout.

`--prometheus <TARGET>` publishes Prometheus text metrics. Use `-` for stdout, a file path for atomic file updates, or `host:port` to serve `/metrics`.
