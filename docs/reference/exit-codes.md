# Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Success, including empty result sets. |
| `1` | Runtime error such as failed export or sampling error. |
| `2` | CLI argument error from Clap. |
| `10` | Default `watch` alert match code. |

Empty filters are not considered runtime errors. A report with no matching rows exits with `0`.

`rescope watch --exit-code <1-255>` can choose a different non-zero code for matched alerts.
