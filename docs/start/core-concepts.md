# Core concepts

## Samples

A sample is one refreshed view of system and process metrics. The sampler stores the elapsed duration between refreshes so CPU core-seconds and rate values can use the actual interval rather than the requested interval.

## Process identity

Processes are tracked by PID, start time and name:

```text
pid + start_time_epoch_s + name
```

This prevents PID reuse from merging two different processes during a recording.

## Grouping

`rescope` can aggregate rows by:

- `process`: individual PID plus process identity
- `name`: process name
- `user`: resolved user name or UID
- `command`: full command line
- `executable`: executable path
- `parent`: parent PID

## Privacy

Command lines can contain secrets. They are hidden by default. Use `--show-command` to display command lines in process rows, or `--group command` when command-line grouping is explicitly needed.
