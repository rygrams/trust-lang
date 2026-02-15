# TRUST Standard Library

Modules available via the `trusty:` import prefix.
The transpiler resolves them automatically — no `Cargo.toml` edits required.

```typescript
import { ... } from "trusty:<module>";
```

---

## Available modules

| Module | Status | Description |
|---|---|---|
| [`trusty:time`](./time.md) | ✅ Implemented | Instant, Duration, SystemTime, Date, Time, DateTime, sleep, compare/add/sub helpers |
| `trusty:fs` | 🔜 Planned | File read/write, directories |
| `trusty:io` | 🔜 Planned | stdin, stdout, stderr |
| `trusty:env` | 🔜 Planned | Environment variables, CLI args |
| `trusty:path` | 🔜 Planned | Path manipulation |
| `trusty:json` | 🔜 Planned | JSON parse/stringify |
| `trusty:rand` | 🔜 Planned | Random numbers, distributions |
| `trusty:math` | 🔜 Planned | sqrt, abs, min, max, clamp, trig |
| `trusty:bd` | 🔜 Planned | SQL (SQLite/Postgres/MySQL), ORM |
| `trusty:http` | 🔜 Planned | HTTP server, router, middleware |
| `trusty:redis` | 🔜 Planned | Cache, pub/sub |
| `trusty:kafka` | 🔜 Planned | Kafka producer/consumer |
| `trusty:amqp` | 🔜 Planned | RabbitMQ, queues |
| `trusty:nats` | 🔜 Planned | Lightweight messaging |
| `trusty:ws` | 🔜 Planned | WebSockets |
| `trusty:grpc` | 🔜 Planned | gRPC services |
| `trusty:graphql` | 🔜 Planned | GraphQL server |
| `trusty:net` | 🔜 Planned | TCP/UDP sockets |
| `trusty:thread` | 🔜 Planned | Threads, channels, mutex |
| `trusty:async` | 🔜 Planned | Async runtime, tasks |
| `trusty:crypto` | 🔜 Planned | SHA2/3, HMAC, AES |
| `trusty:bcrypt` | 🔜 Planned | Password hashing |
| `trusty:jwt` | 🔜 Planned | Token generation/verification |
| `trusty:auth` | 🔜 Planned | OAuth2, sessions, RBAC |
| `trusty:tls` | 🔜 Planned | Certificates, TLS/SSL |
| `trusty:storage` | 🔜 Planned | S3, GCS, Azure Blob |
| `trusty:mail` | 🔜 Planned | SMTP email |
| `trusty:log` | 🔜 Planned | Structured logs |
| `trusty:metrics` | 🔜 Planned | Prometheus counters/gauges |
| `trusty:tracing` | 🔜 Planned | Distributed traces (OpenTelemetry) |
| `trusty:config` | 🔜 Planned | Multi-source config, hot reload |
| `trusty:cli` | 🔜 Planned | CLI argument parser |
| `trusty:process` | 🔜 Planned | Spawn, Command, signals |
| `trusty:docker` | 🔜 Planned | Docker API |
| `trusty:vault` | 🔜 Planned | HashiCorp Vault secrets |
| `trusty:uuid` | 🔜 Planned | UUID generation |
| `trusty:regex` | 🔜 Planned | Regular expressions |
| `trusty:base64` | 🔜 Planned | Encode/decode |
| `trusty:compress` | 🔜 Planned | gzip, zstd, lz4 |
| `trusty:serialize` | 🔜 Planned | Binary serialization |
| `trusty:collections` | 🔜 Planned | Queue, Stack, PriorityQueue |
| `trusty:tensor` | 🔜 Planned | N-dim tensors, SIMD ops |
| `trusty:nn` | 🔜 Planned | Neural network layers |
| `trusty:train` | 🔜 Planned | Training loop, optimizers |
| `trusty:model` | 🔜 Planned | Load/save models (safetensors, GGUF) |
| `trusty:embed` | 🔜 Planned | Vector embeddings, cosine similarity |
| `trusty:linalg` | 🔜 Planned | Matrix, SVD, dot product |
| `trusty:stats` | 🔜 Planned | Mean, variance, regression |
| `trusty:gpu` | 🔜 Planned | CUDA/Metal/WebGPU acceleration |

---

## How it works

When the transpiler encounters `import { X } from "trusty:time"`, it:

1. Strips the import line (does not emit a raw `use` from the source)
2. Looks up `"time"` in the stdlib registry (`stdlib/mod.rs`)
3. Injects the appropriate `use std::...` statements at the top of the generated file
4. Adds any required external crates to the Cargo dependency list
5. Applies API-specific expression mappings (e.g. `Duration.millis(n)` → `Duration::from_millis(...)`)

Adding a new module requires only:
- `crates/trusty-compiler/src/stdlib/<name>.rs` — use statements, crate deps, API mappings
- One entry in `stdlib/mod.rs` `resolve()` match arm
