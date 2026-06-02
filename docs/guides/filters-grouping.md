# Filters and grouping

Filters are combined with AND across filter types and OR within the same filter type.

```bash
rescope record --duration 1m --user postgres --name postgres
rescope snapshot --name node --name bun
rescope snapshot --name-regex '^(node|bun)$' --min-ram 512MiB
rescope live --plain --cmd server.js --min-cpu 5 --invert
```

The first example requires both user and name to match. The second matches either process name.

## Filters

- `--pid <PID>` exact PID, repeatable
- `--user <USER>` user name, UID or `unknown`, repeatable
- `--name <NAME>` case-insensitive process name substring, repeatable
- `--name-regex <REGEX>` case-insensitive process name regex, repeatable
- `--cmd <SUBSTRING>` case-insensitive command-line substring, repeatable
- `--cmd-regex <REGEX>` case-insensitive command-line regex, repeatable
- `--min-cpu <PERCENT>` minimum process CPU percentage
- `--min-ram <SIZE>` minimum resident memory
- `--min-io <SIZE>` minimum read+write delta per sample
- `--invert` excludes rows that match the PID, user, name, command and threshold filters
- `--hide-self` hides the current `rescope` process

## Groups

- `--group process`
- `--group name`
- `--group user`
- `--group command`
- `--group executable`
- `--group parent`

Command grouping intentionally exposes command lines because the user explicitly requested command-line aggregation.
