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
- TUI mode uses an alternate screen with a central `o` options menu, direct menus for sorting/grouping/filtering/view/export, row details, live search, pause/resume, row-limit and interval controls.
- `--once` renders one sample and exits.

Exports are supported only with `--once`.
