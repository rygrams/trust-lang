# TRUST VS Code Syntax Extension

This folder contains a minimal VS Code extension that provides syntax highlighting for TRUST `.trs` files.

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
