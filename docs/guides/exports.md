# Exports

Snapshot, record and `live --once` can export JSON and CSV.

```bash
rescope snapshot --json snapshot.json
rescope snapshot --csv snapshot.csv
rescope record --duration 30s --json report.json --csv report.csv
rescope live --once --json -
```

## Atomic file writes

File exports are written through a temporary file in the destination directory and then renamed into place.

## Stdout

Use `-` to write one export to stdout:

```bash
rescope snapshot --json - | jq '.rows | length'
```

Only one of `--json -` or `--csv -` can be used at a time.
