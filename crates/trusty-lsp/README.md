# trusty-lsp

Minimal Language Server Protocol (LSP) server for TRUST (`.trs`).

## Features

- Diagnostics on open/change (using `trusty-compiler`)
- Completion (keywords, core types, common builtins)
- Hover help for common TRUST tokens

## Run

```bash
cargo run --manifest-path crates/trusty-lsp/Cargo.toml
```

The server communicates over stdio (LSP standard).

## VS Code wiring

Use a VS Code extension that can run an external language server command for a language id.

- language id: `trust`
- command: `cargo`
- args:
  - `run`
  - `--manifest-path`
  - `crates/trusty-lsp/Cargo.toml`
