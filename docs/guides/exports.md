# Exports

Snapshot, record, tree, diff and `live --once` can export JSON and CSV.

```bash
rescope snapshot --json snapshot.json
rescope snapshot --csv snapshot.csv
rescope record --duration 30s --json report.json --csv report.csv
rescope live --once --json -
rescope tree --json tree.json
rescope --json - diff before.json after.json
```

In `rescope live --tui`, press `e` for snapshot exports or `r` for recording exports. The TUI opens a path prompt before writing and refuses to overwrite an existing file.

Recording exports include approximate percentile fields and started/exited process counts. JSON includes bounded timelines; CSV keeps one row per aggregate result.

Continuous live streams use newline-delimited JSON, streaming CSV or Prometheus metrics:

```bash
rescope live --quiet --jsonl live.jsonl
rescope live --quiet --csv-stream live.csv
rescope live --once --quiet --jsonl -
rescope live --prometheus 127.0.0.1:9898
```

`record --raw-samples raw.json` stores replayable raw samples. Use `rescope replay raw.json` to rebuild recording reports with different filters, grouping, sorting or export formats.

## Atomic file writes

File exports are written through a temporary file in the destination directory and then renamed into place.

## Stdout

Use `-` to write one export to stdout:

```bash
rescope snapshot --json - | jq '.rows | length'
```

Only one stdout export can be used at a time. For continuous live streams, use `--quiet` when `--jsonl -`, `--csv-stream -` or `--prometheus -` writes to stdout.
