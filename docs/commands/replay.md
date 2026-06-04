# replay

Rebuild a recording report from raw samples written by `record --raw-samples`.

```bash
rescope replay <RAW-SAMPLES.json> [OPTIONS]
```

## Examples

```bash
rescope record --duration 30s --raw-samples raw.json
rescope replay raw.json --group user --sort cpu-max
rescope replay raw.json --process node --json replay.json --csv replay.csv
```

Replay applies filters, grouping, sorting and exports at replay time. This lets one raw capture answer several follow-up questions without taking another measurement.

Common filters, grouping and exports are documented in [CLI options](../reference/options.md).
