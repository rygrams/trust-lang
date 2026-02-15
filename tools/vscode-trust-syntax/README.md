# TRUST VS Code Syntax Extension

This folder contains a minimal VS Code extension that provides syntax highlighting and snippet autocomplete for TRUST `.trs` files.

## Run in VS Code (extension development host)

1. Open this folder in VS Code:
   `tools/vscode-trust-syntax`
2. Press `F5` to launch an Extension Development Host.
3. Open a `.trs` file and verify highlighting.

## Package and install locally

```bash
cd tools/vscode-trust-syntax
npm install -g @vscode/vsce
vsce package
code --install-extension trust-lang-syntax-0.0.1.vsix
```

## Autocomplete

The extension includes snippet-based autocomplete for common TRUST patterns:
- `fn`, `main`, `if`, `ife`, `tern`, `match`
- `try`, `tryf`
- `forc`, `forin`, `forof`, `loop`
- `val`, `var`, `const`
- `imp`, `expfn`, `expconst`, `struct`, `expstruct`, `enum`, `impl`
- `cw`
- casts like `string`, `boolean`, `int32`, `float64`

IntelliSense level:
- Available now: snippet completion + VS Code word-based suggestions.
- Not implemented yet: semantic completion (symbols/types across files), diagnostics, go-to-definition.
- For full IntelliSense, a Language Server (LSP) is needed.

## Colors for types vs keywords

Yes, type tokens and keyword tokens are separated in the grammar:
- Types use `storage.type.*`
- Keywords use `keyword.*` / `storage.modifier.*`

The final colors depend on the active VS Code theme.  
Most themes color them differently by default; if not, users can customize token colors in their settings.
