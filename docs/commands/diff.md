# diff

Compare two JSON reports and rank changed rows.

```bash
rescope diff <BEFORE.json> <AFTER.json> [OPTIONS]
```

## Examples

```bash
rescope snapshot --json before.json
rescope snapshot --json after.json
rescope diff before.json after.json
rescope --json - diff before.json after.json
rescope --csv diff.csv diff before.json after.json
```

## Compared fields

Rows are matched by group type, PID when present and display name. `diff` reports added, removed and changed rows using CPU, RAM and combined I/O deltas.

Use `--all` to keep every changed row or `--limit <N>` to cap output.
