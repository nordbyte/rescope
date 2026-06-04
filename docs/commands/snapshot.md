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
rescope snapshot --profile tree --parent-name systemd
rescope snapshot --exe /usr/bin --profile io
rescope snapshot --json snapshot.json
rescope snapshot --no-system --limit 5
```

## Options

Common filters, grouping and sort options are documented in [CLI options](../reference/options.md).
Profiles and JSON config defaults are also documented there.

Snapshot-specific options:

- `--interval <DURATION>` warm-up and CPU measurement interval, default `1s`
- `--show-system` show system summary
- `--no-system` hide system summary
- `--all` do not truncate rows
- `--normalize-cpu` display CPU divided by logical CPU count
