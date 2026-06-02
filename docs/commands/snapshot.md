# snapshot

Take one measured snapshot and exit.

```bash
rescope snapshot [OPTIONS]
```

## Examples

```bash
rescope snapshot --limit 20
rescope snapshot --group user --sort ram --limit 10
rescope snapshot --group executable --sort io --all
rescope snapshot --json snapshot.json
```

## Options

Common filters, grouping and sort options are documented in [CLI options](../reference/options.md).

Snapshot-specific options:

- `--interval <DURATION>` warm-up and CPU measurement interval, default `1s`
- `--show-system` show system summary
- `--all` do not truncate rows
- `--normalize-cpu` display CPU divided by logical CPU count
