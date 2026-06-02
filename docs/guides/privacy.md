# Privacy

`rescope` is read-only. It does not kill processes, modify system state, send telemetry or make network requests.

## Command lines

Command lines may contain tokens, passwords or connection strings. Process rows show only the process name by default.

Use `--show-command` when full commands are needed:

```bash
rescope snapshot --show-command --pid 1234
```

`--cmd` filters command lines internally without automatically printing them.

`--group command` displays command lines because command aggregation is the explicit purpose of that mode.
