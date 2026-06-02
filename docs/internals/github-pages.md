# GitHub Pages

The public documentation is built with VitePress from `docs/`.

## Workflow

`.github/workflows/pages.yml` runs on pushes to `main` and manual dispatch.

It performs:

1. Checkout.
2. Node.js setup.
3. Docs dependency install.
4. Docs consistency check.
5. VitePress build.
6. Pages artifact upload.
7. Pages deployment.

The deployed site is configured for:

```text
https://nordbyte.github.io/rescope/
```
