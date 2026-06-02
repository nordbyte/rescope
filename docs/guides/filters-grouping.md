# Filters and grouping

Filters are combined with AND across filter types and OR within the same filter type.

```bash
rescope record --duration 1m --user postgres --name postgres
rescope snapshot --name node --name bun
rescope snapshot --name-regex '^(node|bun)$' --min-ram 512MiB
rescope live --plain --cmd server.js --min-cpu 5 --invert
rescope snapshot --exe /usr/bin --parent-name systemd
rescope live --profile tree --tui
```

The first example requires both user and name to match. The second matches either process name.

## Filters

- `--pid <PID>` exact PID, repeatable
- `--user <USER>` user name, UID or `unknown`, repeatable
- `--name <NAME>` case-insensitive process name substring, repeatable
- `--name-regex <REGEX>` case-insensitive process name regex, repeatable
- `--cmd <SUBSTRING>` case-insensitive command-line substring, repeatable
- `--cmd-regex <REGEX>` case-insensitive command-line regex, repeatable
- `--exe <SUBSTRING>` case-insensitive executable-path substring, repeatable
- `--exe-regex <REGEX>` case-insensitive executable-path regex, repeatable
- `--parent <PID>` exact parent PID, repeatable
- `--parent-name <NAME>` case-insensitive parent process name substring, repeatable
- `--parent-regex <REGEX>` case-insensitive parent process name regex, repeatable
- `--min-cpu <PERCENT>` minimum process CPU percentage
- `--min-ram <SIZE>` minimum resident memory
- `--min-io <SIZE>` minimum read+write delta per sample
- `--invert` excludes rows that match active PID, user, name, command, executable, parent and threshold filters
- `--hide-self` hides the current `rescope` process

## Groups

- `--group process`
- `--group name`
- `--group user`
- `--group command`
- `--group executable`
- `--group parent`

Parent grouping displays parent PID plus parent process name when available, for example `1 (systemd)`. Command grouping intentionally exposes command lines because the user explicitly requested command-line aggregation.

## Profiles

Profiles are shortcuts for common workflows:

- `--profile cpu`
- `--profile memory`
- `--profile io`
- `--profile commands`
- `--profile users`
- `--profile tree`

For example, `rescope snapshot --profile io --limit 10` selects process rows sorted by combined I/O, while `rescope live --profile tree --tui` starts an interactive parent-grouped view.
