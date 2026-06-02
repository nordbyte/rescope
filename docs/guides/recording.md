# Recording reports

`rescope record` measures a bounded window and prints an aggregate report at the end.

```bash
rescope record --duration 1m --interval 1s --sort io --limit 20
rescope record --duration 30s --profile memory --include-idle
```

Recordings are aggregated while samples arrive. Long runs keep bounded timelines and percentile inputs instead of retaining every raw sample.

## What is aggregated

Recording rows include:

- Average and max CPU
- Approximate CPU p95 and p99
- CPU core-seconds
- RAM start, end, min, max, p95, average and delta
- Total read, write and combined I/O
- Approximate p95 combined I/O per sample
- Average read/write/I/O rate
- Started and exited process identity counts per aggregate row
- First and last seen timestamps
- Lifecycle status
- RAM, CPU, read and write timelines in JSON

## Lifecycle status

`lifecycle_status` can be:

- `observed_full_duration`
- `started_during_recording`
- `exited_during_recording`
- `started_and_exited_during_recording`

## Idle rows

By default, rows with no CPU, I/O or RAM movement are hidden from recordings. Use `--include-idle` to include them without changing the row limit. Use `--all` to include idle rows and disable the row limit.
