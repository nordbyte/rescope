# tree

Render matching processes as a parent-child tree with subtree totals.

```bash
rescope tree [OPTIONS]
```

## Examples

```bash
rescope tree --limit 50
rescope tree --process postgres --show-path
rescope tree --parent-name systemd --sort ram
rescope tree --json tree.json
rescope tree --csv tree.csv
```

## Notes

Tree rows include direct CPU, RAM and I/O plus subtree CPU, subtree RAM and subtree I/O. Children are sorted within their parent by the selected sort metric.

Common filters and output flags are documented in [CLI options](../reference/options.md).
