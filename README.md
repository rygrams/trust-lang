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

Create a file `hello.trust`:

```typescript
function greet(name: string): string {
  return `Hello, ${name}!`;
}

function main() {
  console.log(greet("World"));
}
```

Compile and run:

```bash
trusty run hello.trust
```

## Documentation

See [docs/](./docs/) for more information.

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
cargo run -p trusty-cli -- examples/fibonacci.trust
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
