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
- timeline arrays for recording rows

Times are serialized as Unix milliseconds.

## CSV

CSV exports contain the visible aggregate rows without terminal sparklines. CPU raw and normalized columns are both included.

## Stdout

Use `-` as the path for one export:

```bash
rescope snapshot --json -
```
