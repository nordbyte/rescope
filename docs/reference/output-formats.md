# Output formats

## Terminal tables

Terminal tables are optimized for scanning. Use `--bytes` for raw bytes and `--normalize-cpu` for normalized CPU display.

## JSON

JSON exports include:

- tool and version envelope
- report metadata
- filters
- grouped rows
- notes
- bounded timeline arrays for recording rows
- recording percentile and lifecycle count fields
- process details when available

Times are serialized as Unix milliseconds.

## CSV

CSV exports contain the visible aggregate rows without terminal sparklines. CPU raw and normalized columns are both included. Recording CSV also includes CPU p95/p99, RAM p95, I/O p95, `started_count`, `exited_count` and process detail columns.

`tree` CSV exports a flattened tree with a `depth` column. `diff` CSV exports before/after CPU, RAM and I/O values plus deltas.

## JSONL and streaming CSV

`live --jsonl` writes one compact JSON object per sample. `live --csv-stream` writes a header once and appends snapshot rows as samples arrive.

## Stdout

Use `-` as the path for one export:

```bash
rescope snapshot --json -
```
