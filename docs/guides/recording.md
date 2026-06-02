# Recording reports

`rescope record` measures a bounded window and prints an aggregate report at the end.

```bash
rescope record --duration 1m --interval 1s --sort io --limit 20
```

## What is aggregated

Recording rows include:

- Average and max CPU
- CPU core-seconds
- RAM start, end, min, max, average and delta
- Total read, write and combined I/O
- Average read/write/I/O rate
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
