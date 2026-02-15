# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Project Is

TRUST (`.trs`) is an experimental TypeScript-like language that transpiles to Rust. It provides TypeScript syntax while targeting Rust's type system, ownership model, and zero-cost abstractions — no JavaScript runtime.

## Commands

```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p trusty-compiler
cargo test -p trusty-cli

# Run the CLI directly (without installing)
cargo run -p trusty-cli -- examples/hello.trs

# Install the CLI globally
cargo install --path crates/trusty-cli

# CLI usage after install
trusty build input.trs           # Transpile to .rs
trusty build input.trs --compile # Transpile and compile to binary
trusty run input.trs             # Transpile, compile, and execute
trusty check input.trs           # Syntax check only
```

## Architecture

The project is a Cargo workspace with two crates:

- **`crates/trusty-compiler`** — library crate; the core transpiler
- **`crates/trusty-cli`** — binary crate; wraps the library with a CLI (`clap`)

### Compilation Pipeline

```
.trs source
  → parser.rs        (SWC parses TypeScript syntax into an AST)
  → transpiler/      (walks the AST and emits Rust source)
      mod.rs         orchestrates; iterates top-level declarations
      functions.rs   function declarations and bodies
      expressions.rs binary ops, identifiers, template literals, calls
      statements.rs  return and expression statements
      types.rs       TypeScript → Rust type mapping
  → codegen.rs       writes Rust source to disk
  → rustc (optional) compiles generated Rust to a binary
```

### Public API (`lib.rs`)

```rust
pub fn compile(source: &str) -> Result<String>
pub fn compile_formatted(source: &str) -> Result<String>
```

### Type Mapping (`types.rs`)

| TRUST          | Rust   |
|---------------|--------|
| `int`         | `i32`  |
| `int8`        | `i8`   |
| `int16`       | `i16`  |
| `int32`       | `i32`  |
| `int64`       | `i64`  |
| `float`       | `f64`  |
| `float32`     | `f32`  |
| `float64`     | `f64`  |
| `number`      | `i32` (deprecated alias) |
| `string`      | `String` |
| `boolean`     | `bool` |

### Notable Transpilation Behaviors

- Template literals (`` `Hello, ${name}!` ``) → `format!("Hello, {}!", name)`
- `console.write(...)` → `println!(...)`
- Other member expression calls → `.method()` Rust calls

### Current Limitations

No support yet for: variable declarations, loops, conditionals, classes/interfaces, or a module system. Only function declarations with return/expression statements are supported.

## Key Dependencies

- **`swc_ecma_parser`** — TypeScript/JS parser (same as used by Next.js)
- **`anyhow` / `thiserror`** — error handling
- **`clap`** — CLI argument parsing
- **`pretty_assertions`** — improved test failure output
