# completions

Generate shell completions for local installation.

```bash
rescope completions <SHELL> [OPTIONS]
```

## Examples

```bash
rescope completions bash > rescope.bash
rescope completions zsh --output _rescope
rescope completions fish --output rescope.fish
```

Supported shells are provided by Clap and include Bash, Zsh, Fish, PowerShell and Elvish.
