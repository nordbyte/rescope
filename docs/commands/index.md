# CLI command reference

`rescope` has runtime commands, replay and packaging helpers plus standard help and version output.

| Command | Purpose |
| --- | --- |
| [`snapshot`](snapshot.md) | One-shot non-interactive sample. |
| [`live`](live.md) | Repeated live view, plain or interactive. |
| [`record`](record.md) | Bounded measurement with final report. |
| [`replay`](replay.md) | Rebuild a recording report from raw samples. |
| [`tree`](tree.md) | Parent-child process tree with subtree totals. |
| [`watch`](watch.md) | Alert-style polling that exits when filters match. |
| [`diff`](diff.md) | Compare two JSON reports and rank changed rows. |
| [`completions`](completions.md) | Generate shell completions. |
| [`man`](man.md) | Generate a man page. |
| [`help and version`](help-version.md) | Built-in help and version flags. |

Running `rescope` without a subcommand is equivalent to `rescope live`.
