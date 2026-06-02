# Live monitoring

`rescope live` repeatedly samples system processes and renders the current top rows.

## Plain mode

```bash
rescope live --plain --interval 1s --limit 20
```

Plain mode clears and redraws the terminal. It is useful in simple terminals and over remote shells.

## Interactive mode

```bash
rescope live --tui --group user --sort cpu
```

Interactive mode uses an alternate screen and exits with `q`, `Esc` or `Ctrl-C`.

While the TUI is running, press `c` for CPU, `m` for RAM, `i` for combined I/O, `r` for reads, `w` for writes, `p` for PID, `n` for name or `u` for user sorting.

## One-shot live

`live --once` is equivalent to a live-rendered snapshot and can export:

```bash
rescope live --once --json live.json
rescope live --once --csv -
```

Continuous live exports are intentionally rejected because a single JSON or CSV file would not have a stable shape.
