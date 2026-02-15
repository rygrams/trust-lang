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

| TRUST     | Rust                     |
| --------- | ------------------------ |
| `int`     | `i32`                    |
| `int8`    | `i8`                     |
| `int16`   | `i16`                    |
| `int32`   | `i32`                    |
| `int64`   | `i64`                    |
| `float`   | `f64`                    |
| `float32` | `f32`                    |
| `float64` | `f64`                    |
| `number`  | `i32` (deprecated alias) |
| `string`  | `String`                 |
| `boolean` | `bool`                   |

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

## Vision

- TRUST like Go / Elixir, but zero-cost
- The idea is to have a batteries-included language where you never need to look for an external crate for common use cases.

## TRUST Standard Library (`trusty:*`)

Imports with the `trusty:` prefix are native modules managed by the transpiler. The transpiler resolves them to the appropriate Rust crates, injects `use` statements, and adds Cargo dependencies automatically. Unused symbols are eliminated at compile time — no runtime overhead.

### Import resolution levels

```
trusty:http     →  native TRUST module  (stable API, managed by transpiler)
tiny_http       →  raw Rust crate        (escape hatch, direct Rust API)
./my-module     →  local .trs file       (user module)
```

### Web & Network

| Module | Description | Rust crate |
|---|---|---|
| `trusty:http` | HTTP server, router, middleware, request/response | actix-web |
| `trusty:ws` | Real-time WebSockets | tokio-tungstenite |
| `trusty:grpc` | gRPC services, protobuf | tonic |
| `trusty:graphql` | GraphQL server | async-graphql |
| `trusty:net` | Low-level TCP/UDP sockets | std::net |
| `trusty:dns` | DNS resolution | hickory-dns |
| `trusty:http-client` | Outgoing HTTP requests | reqwest |

### Database & Cache

| Module | Description | Rust crate |
|---|---|---|
| `trusty:bd` | Unified SQL (SQLite/Postgres/MySQL), lightweight ORM | sqlx |
| `trusty:redis` | Cache, pub/sub, data structures | redis-rs |
| `trusty:mongo` | NoSQL documents | mongodb |
| `trusty:elastic` | Full-text search | elasticsearch-rs |
| `trusty:kv` | Embedded key-value store | sled |

### Messaging & Microservices

| Module | Description | Rust crate |
|---|---|---|
| `trusty:kafka` | Kafka producer/consumer | rdkafka |
| `trusty:amqp` | RabbitMQ, queues, exchanges | lapin |
| `trusty:nats` | Lightweight messaging, distributed pub/sub | nats.rs |
| `trusty:microservice` | Service discovery, RPC, load balancing | tonic + tower |

### System & Files

| Module | Description | Rust crate |
|---|---|---|
| `trusty:fs` | File read/write, directories | std::fs |
| `trusty:path` | Path manipulation | std::path |
| `trusty:env` | Environment variables, CLI args | std::env |
| `trusty:process` | Spawn, Command, signals | std::process |
| `trusty:storage` | S3, GCS, Azure Blob | opendal |
| `trusty:watch` | File system watching, hot reload | notify |

### Concurrency & Async

| Module | Description | Rust crate |
|---|---|---|
| `trusty:thread` | Threads, channels, mutex | std::thread |
| `trusty:async` | Async runtime, tasks, futures | tokio |
| `trusty:channel` | MPSC, broadcast, oneshot | tokio::sync |
| `trusty:atomic` | Atomic types, lock-free | std::sync::atomic |

### Security & Auth

| Module | Description | Rust crate |
|---|---|---|
| `trusty:crypto` | SHA2/3 hash, HMAC, AES | sha2 + aes |
| `trusty:bcrypt` | Password hashing | bcrypt |
| `trusty:jwt` | Token generation/verification | jsonwebtoken |
| `trusty:auth` | OAuth2, sessions, RBAC | oxide-auth |
| `trusty:tls` | Certificates, TLS/SSL | rustls |
| `trusty:vault` | HashiCorp Vault secrets | vaultrs |

### Data & Formats

| Module | Description | Rust crate |
|---|---|---|
| `trusty:json` | JSON parse/stringify | serde_json |
| `trusty:toml` | TOML config | toml-rs |
| `trusty:yaml` | YAML parse | serde_yaml |
| `trusty:csv` | CSV read/write | csv |
| `trusty:xml` | XML parse | quick-xml |
| `trusty:base64` | Encode/decode | base64 |
| `trusty:uuid` | UUID generation | uuid |
| `trusty:regex` | Regular expressions | regex |

### Observability & DevOps

| Module | Description | Rust crate |
|---|---|---|
| `trusty:log` | Structured logs, log levels | tracing |
| `trusty:metrics` | Counters, gauges, histograms (Prometheus) | metrics |
| `trusty:tracing` | Distributed traces (OpenTelemetry) | opentelemetry |
| `trusty:config` | Multi-source config, hot reload | config-rs |
| `trusty:docker` | Docker API | bollard |
| `trusty:health` | Health checks, readiness probes | — |

### Utilities

| Module | Description | Rust crate |
|---|---|---|
| `trusty:time` | Date, duration, timezone, sleep | chrono + std |
| `trusty:rand` | Random numbers, distributions | rand |
| `trusty:math` | sqrt, abs, min, max, clamp, trig | std builtins |
| `trusty:cli` | CLI argument parser, subcommands | clap |
| `trusty:mail` | SMTP email sending | lettre |
| `trusty:collections` | Queue, Stack, PriorityQueue, BTreeMap | std::collections |
| `trusty:test` | Unit tests, assertions, mocks | cfg(test) + mockall |
| `trusty:bench` | Benchmarks | criterion |
| `trusty:serialize` | Compact binary serialization | bincode |
| `trusty:compress` | gzip, zstd, lz4 | flate2 + zstd |

### Vector Compute & Machine Learning

| Module | Description | Rust crate |
|---|---|---|
| `trusty:tensor` | N-dim tensors, SIMD-vectorized ops | candle-core |
| `trusty:nn` | Neural network layers, activations | candle-nn |
| `trusty:train` | Training loop, optimizers, loss functions | candle-core |
| `trusty:model` | Load/save models (GGUF, safetensors) | candle + hf-hub |
| `trusty:tokenizer` | BPE/WordPiece tokenization | tokenizers |
| `trusty:embed` | Vector embeddings, cosine similarity | candle-nn |
| `trusty:linalg` | Matrix, vector, dot product, SVD | nalgebra |
| `trusty:stats` | Mean, variance, correlation, regression | statrs |
| `trusty:gpu` | CUDA/Metal/WebGPU acceleration | wgpu + candle |
