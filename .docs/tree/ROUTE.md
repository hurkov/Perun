# Project Tree Route

This folder is a documentation mirror of the project root. It contains the
same meaningful folders, while implementation/configuration files are replaced
with Markdown files named `<real-file>.<extension>.md`.

This route describes only the contents of `.docs/tree/` itself.

| Entry | Description |
|---|---|
| `src/` | Mirror of the Rust source tree; see `src/ROUTE.md`. |
| `.bruno/` | Mirror of the Bruno API collection; see `.bruno/ROUTE.md`. |
| `.idea/` | Mirror of IDE metadata; see `.idea/ROUTE.md`. |
| `soundbank/` | Mirror of the local-development sound storage fallback/override (`./soundbank` or `PERUN_DATA_DIR=./soundbank`); see `soundbank/ROUTE.md`. |
| `Cargo.md` | Description of the real `Cargo.toml`. |
| `Cargo.lock.md` | Description of the real `Cargo.lock`. |
| `CLAUDE.md` | Description of the real `CLAUDE.md`. |
| `.gitignore.md` | Description of the real `.gitignore`. |

Generated `target/` output is intentionally not mirrored file-by-file.
