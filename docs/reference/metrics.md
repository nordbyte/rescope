# Metrics

## CPU

CPU is sampled as process CPU percentage from `sysinfo`. Values can exceed `100%` when a process uses multiple cores.

Recording reports calculate CPU core-seconds with the actual elapsed sample interval and include approximate p95/p99 CPU values:

```text
cpu_core_seconds += (cpu_percent / 100) * actual_sample_interval_seconds
```

`--normalize-cpu` affects display values by dividing by logical CPU count. Core-seconds remain raw.

## RAM

RAM is resident memory when the platform exposes that metric. Recording reports track start, end, min, max, p95, average and delta.

## Disk I/O

Read and write totals are calculated from per-process counters. The first appearance of a process identity has zero delta so pre-existing I/O is not counted as recording activity. Recording reports include approximate p95 combined I/O per sample.

## Lifecycle counts

Recording rows include `started_count` and `exited_count`. For process rows these count the selected process identity. For grouped rows they count process identities that first appeared after the recording started or disappeared before it ended.

## Process details

When the platform exposes them, process rows include status, runtime, accumulated CPU time, thread count, open file count and Linux cgroup path. Missing fields are omitted from JSON and left empty in CSV.

## Percentile accuracy

Timeline and percentile inputs are bounded for long runs. Percentiles remain representative but can be approximate after very long recordings because old points are downsampled.

## Platform notes

On Windows, per-process I/O may include non-disk I/O. On Unix-like systems, cached operations may not increase physical disk counters.
