# Troubleshooting

## CPU values look too high

Process CPU can exceed `100%` on multi-core systems. Use `--normalize-cpu` to display process CPU as a share of all logical CPUs.

## Disk I/O is zero

Per-process disk counters are platform-dependent. Cached operations on Unix-like systems may not appear as physical disk reads or writes. On Windows, counters may include non-disk I/O depending on OS APIs.

## User names are numeric

If the platform cannot resolve a user name, `rescope` falls back to UID strings. Filtering by either user name or UID is supported.

## `--json -` and `--csv -` fail together

Only one export can write to stdout at a time. Use files if both formats are needed:

```bash
rescope snapshot --json snapshot.json --csv snapshot.csv
```

## TUI does not fit the terminal

Use the plain renderer:

```bash
rescope live --plain --limit 10
```
