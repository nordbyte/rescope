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

The interactive TUI is menu-driven:

- `o` opens the central options menu for sorting, grouping, filters, view settings, sampling, recording, export, details and help.
- `?` opens the help overlay.
- `s` opens sorting directly.
- `g` opens grouping directly.
- `f` opens filters directly.
- `v` opens view settings directly.
- `r` opens recording controls directly.
- `e` opens snapshot export directly.
- `/` edits the live search query.
- Up/down selects table rows or moves within menus.
- PageUp/PageDown moves the selected table row by larger steps.
- `Enter` opens details for the selected row or applies the selected menu item.
- In details, `f` toggles between frozen details and following the same process or group identity.
- `Space` pauses or resumes sampling.
- `+` and `-` adjust the visible row limit.
- `[` and `]` adjust the refresh interval.
- `n`, `b` and `c` toggle normalized CPU, raw bytes and command display.

Menus use up/down plus Enter. Press `Esc` to close an overlay; from the main view it exits the TUI. In search and export path prompts, regular characters including `q` are inserted as text; use `Esc` or `Ctrl-C` to leave those prompts.

The view menu can hide optional PID, user, rate, total and top-process columns. Narrow terminals also hide low-priority columns automatically so the selected row, primary name, CPU and RAM columns remain readable.

The filter menu can set live search, invert active filters, hide the current `rescope` process and cycle CPU, RAM and I/O threshold presets without restarting the command.

The recording menu starts a short streaming recording from inside the TUI, stops it early if needed and exports the last recording as JSON or CSV. Export actions open a path prompt and refuse to overwrite an existing file.

## One-shot live

`live --once` is equivalent to a live-rendered snapshot and can export:

```bash
rescope live --once --json live.json
rescope live --once --csv -
```

Continuous live exports are intentionally rejected because a single JSON or CSV file would not have a stable shape.
