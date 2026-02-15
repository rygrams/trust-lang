# TRUST Language

> ⚠️ Experimental project: TRUST is currently in active development and not yet production-ready.

Trust (trust-lang) is a TypeScript-like language that compiles to Rust and produces native binaries. It keeps a familiar TS-style syntax while adopting Rust’s ownership model and zero-cost abstractions to deliver near-Rust performance without a JavaScript runtime.

## Features

- 🚀 TypeScript-like syntax
- ⚡ Compiles to native Rust code
- 🔥 Zero runtime overhead
- 📦 Full access to Rust crates ecosystem
- 🛡️ Memory safe by design

## Installation

Prerequisites:

- Rust toolchain installed (`rustup`, `cargo`)

From source (this repository):

```bash
# Clone
git clone https://github.com/you/trust-lang
cd trust-lang

# Install the CLI binary
cargo install --path crates/trusty-cli
```

Verify installation:

```bash
trusty --help
```

## Quick Start

Create a file `hello.trs`:

```typescript
function greet(name: string): string {
  return `Hello, ${name}!`;
}

function main() {
  console.write(greet("World"));
}
```

Compile and run:

```bash
trusty run hello.trs
```

Format source:

```bash
trusty format hello.trs
trusty format hello.trs --check
```

Struct example:

```bash
trusty run examples/struct-point.trs
```

## Documentation

See [docs/](./docs/) for more information.

## VS Code Syntax Highlighting

A minimal VS Code syntax extension for `.trs` is available in:
`tools/vscode-trust-syntax`

## LSP (IntelliSense)

A minimal TRUST language server is available in:
`crates/trusty-lsp`

## Imports and Modules

Current behavior is not full TypeScript module resolution.

- `import { X } from "crate/path"` is transpiled to Rust `use crate::path::X;`
- Local modules are supported with `./` and `../` in CLI build/check/run:
  - `import { add } from "./math";`
  - `import { Point } from "../models/point";`
- Supported direct exports in local `.trs` files:
  - `export const ...`
  - `export function ...`
  - `export struct ...`
  - `export enum ...`
  - `export implements Name { export function ... }`
- External crates can be declared in `trusty.json` and used by `trusty build/run`
- Not supported yet:
  - `export * from "./x"`
  - `export { a, b } from "./x"` / mapped export lists
  - `export default ...`

Try the module example:

```bash
trusty run examples/modules/main.trs
```

## Development

```bash
# Clone the repo
git clone https://github.com/you/trust-lang
cd trust-lang

# Build the workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Run CLI
cargo run -p trusty-cli -- examples/main.trs
```

## Project Structure

```
trust-lang/
├── crates/
│   ├── trusty-compiler/   # Core transpiler library
│   └── trusty-cli/        # CLI executable
├── examples/              # Example TRUST code
└── docs/                  # Documentation
```

## License

MIT OR Apache-2.0
