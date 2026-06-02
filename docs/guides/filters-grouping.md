# Filters and grouping

Filters are combined with AND across filter types and OR within the same filter type.

```bash
rescope record --duration 1m --user postgres --name postgres
rescope snapshot --name node --name bun
```

The first example requires both user and name to match. The second matches either process name.

## Filters

- `--pid <PID>` exact PID, repeatable
- `--user <USER>` user name, UID or `unknown`, repeatable
- `--name <NAME>` case-insensitive process name substring, repeatable
- `--cmd <SUBSTRING>` case-insensitive command-line substring, repeatable
- `--hide-self` hides the current `rescope` process

## Groups

- `--group process`
- `--group name`
- `--group user`
- `--group command`
- `--group executable`
- `--group parent`

Command grouping intentionally exposes command lines because the user explicitly requested command-line aggregation.
