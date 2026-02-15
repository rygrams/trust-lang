# TRUST VS Code Extension

This folder contains a VS Code extension for TRUST `.trs` files:
- syntax highlighting
- snippet autocomplete
- LSP integration (diagnostics, completion, hover) via `crates/trusty-lsp`

## Run in VS Code (extension development host)

1. Open this folder in VS Code:
   `tools/vscode-trust-syntax`
2. Press `F5` to launch an Extension Development Host.
3. Install extension dependencies:
   ```bash
   cd tools/vscode-trust-syntax
   npm install
   ```
4. Press `F5` to launch an Extension Development Host.
5. Open a `.trs` file and verify:
   - highlighting/snippets
   - diagnostics/completion/hover from LSP

## Package and install locally

```bash
cd tools/vscode-trust-syntax
npm install -g @vscode/vsce
vsce package
code --install-extension trust-lang-syntax-0.0.1.vsix
```

## IntelliSense

The extension includes snippet autocomplete for common TRUST patterns:
- `fn`, `main`, `if`, `ife`, `tern`, `match`
- `try`, `tryf`
- `forc`, `forin`, `forof`, `loop`
- `val`, `var`, `const`
- `imp`, `expfn`, `expconst`, `struct`, `expstruct`, `enum`, `impl`
- `cw`
- casts like `string`, `boolean`, `int32`, `float64`

Semantic IntelliSense is provided by `trusty-lsp` when it starts successfully.

Default LSP launch:
- command: `cargo`
- args: `run --manifest-path crates/trusty-lsp/Cargo.toml -- --stdio`
- cwd: first workspace folder

If your workspace root is not the repository root, configure:
- `trust.languageServer.cwd`
- or `trust.languageServer.command` / `trust.languageServer.args`

You can restart LSP with command:
- `TRUST: Restart Language Server`
- `TRUST: Setup Workspace (LSP/Format/Lint)` writes `.vscode/settings.json` defaults.

## Format on Save

The extension now registers a TRUST formatter provider.
- It uses `trust.format.command` + `trust.format.args`
- Default: `cargo run --manifest-path crates/trusty-cli/Cargo.toml -- format ${file}`
- `editor.formatOnSave` is enabled for `[trust]` by default.

## Lint on Save

Lint/check can run automatically on save:
- setting: `trust.lint.onSave` (default: `true`)
- command: `trust.lint.command` + `trust.lint.args`
- default: `cargo run --manifest-path crates/trusty-cli/Cargo.toml -- check ${file}`

## Colors for types vs keywords

Yes, type tokens and keyword tokens are separated in the grammar:
- Types use `storage.type.*`
- Keywords use `keyword.*` / `storage.modifier.*`

The final colors depend on the active VS Code theme.  
Most themes color them differently by default; if not, users can customize token colors in their settings.
