# Privacy

`rescope` is read-only. It does not kill processes, modify system state, send telemetry or make network requests.

## Command lines

Command lines may contain tokens, passwords or connection strings. Process rows show only the process name by default.

Use `--show-command` when full commands are needed:

```bash
rescope snapshot --show-command --pid 1234
```

`--cmd` and `--cmd-regex` filter command lines internally without automatically printing them.

`--group command` displays command lines because command aggregation is the explicit purpose of that mode.

## Executable paths

Executable paths are less sensitive than command lines, but they can still expose user names or local project paths. Process rows hide them by default.

Use `--show-path` when full executable paths are needed:

```bash
rescope snapshot --process postgres --show-path
```

`--path` and `--exe` filter executable paths internally. `--process` also searches executable paths and command lines internally without automatically printing commands.

The interactive TUI collects command-line or executable details only when a current view, filter, search, path display or recording needs them.
