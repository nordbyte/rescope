# live

Render repeated snapshots.

```bash
rescope live [OPTIONS]
```

## Examples

```bash
rescope live
rescope live --plain --interval 2s
rescope live --tui --group command --sort cpu
rescope live --once --json -
```

## Modes

- Plain mode clears and redraws the terminal.
- TUI mode uses an alternate screen, opens a sort menu with `s`, applies the selected sort with Enter and exits with `q`, `Esc` or `Ctrl-C`.
- `--once` renders one sample and exits.

Exports are supported only with `--once`.
