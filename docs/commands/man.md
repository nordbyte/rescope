# man

Generate a roff man page for local packaging.

```bash
rescope man [OPTIONS]
```

## Examples

```bash
rescope man > rescope.1
rescope man --output rescope.1
```

The generated page is based on the same Clap command metadata as `rescope --help`.
