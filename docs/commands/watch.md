# watch

Poll until filters match and then exit with an alert code.

```bash
rescope watch [OPTIONS]
```

## Examples

```bash
rescope watch --name postgres --min-cpu 80 --duration 5m
rescope watch --name postgres --min-cpu 80 --for 30s --duration 5m
rescope watch --process node --min-ram 1GiB --exit-code 20
rescope watch --cmd server.js --stream --duration 30s
rescope watch --name definitely-no-such-process --duration 5s
```

## Behavior

`watch` exits with `0` when no rows match before the duration ends. When rows match, it prints the matching snapshot and exits with `--exit-code`, default `10`.

Use `--for <DURATION>` to require a continuous match before alerting. Use `--stream` to print every matching sample until the duration ends; without `--stream`, the first sustained match exits immediately.

Common filters and output flags are documented in [CLI options](../reference/options.md).
