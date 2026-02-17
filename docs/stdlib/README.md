# TRUST Standard Library

Modules are imported with the `trusty:` prefix.

```typescript
import { ... } from "trusty:<module>";
```

## Modules Index

| Module | Status | Doc |
|---|---|---|
| `trusty:time` | ✅ Implemented | [time.md](./time.md) |
| `trusty:math` | ✅ Implemented | [math.md](./math.md) |
| `trusty:rand` | ✅ Implemented | [rand.md](./rand.md) |
| `trusty:json` | ✅ Implemented | [json.md](./json.md) |
| `trusty:http` | ✅ Implemented | [http.md](./http.md) |
| `trusty:fs` | 🔜 Planned | [fs.md](./fs.md) |
| `trusty:io` | 🔜 Planned | [io.md](./io.md) |
| `trusty:env` | 🔜 Planned | [env.md](./env.md) |
| `trusty:path` | 🔜 Planned | [path.md](./path.md) |
| `trusty:bd` | 🔜 Planned | [bd.md](./bd.md) |
| `trusty:redis` | 🔜 Planned | [redis.md](./redis.md) |
| `trusty:kafka` | 🔜 Planned | [kafka.md](./kafka.md) |
| `trusty:amqp` | 🔜 Planned | [amqp.md](./amqp.md) |
| `trusty:nats` | 🔜 Planned | [nats.md](./nats.md) |
| `trusty:ws` | 🔜 Planned | [ws.md](./ws.md) |
| `trusty:grpc` | 🔜 Planned | [grpc.md](./grpc.md) |
| `trusty:graphql` | 🔜 Planned | [graphql.md](./graphql.md) |
| `trusty:net` | 🔜 Planned | [net.md](./net.md) |
| `trusty:thread` | 🔜 Planned | [thread.md](./thread.md) |
| `trusty:async` | 🔜 Planned | [async.md](./async.md) |
| `trusty:crypto` | 🔜 Planned | [crypto.md](./crypto.md) |
| `trusty:bcrypt` | 🔜 Planned | [bcrypt.md](./bcrypt.md) |
| `trusty:jwt` | 🔜 Planned | [jwt.md](./jwt.md) |
| `trusty:auth` | 🔜 Planned | [auth.md](./auth.md) |
| `trusty:tls` | 🔜 Planned | [tls.md](./tls.md) |
| `trusty:storage` | 🔜 Planned | [storage.md](./storage.md) |
| `trusty:mail` | 🔜 Planned | [mail.md](./mail.md) |
| `trusty:log` | 🔜 Planned | [log.md](./log.md) |
| `trusty:metrics` | 🔜 Planned | [metrics.md](./metrics.md) |
| `trusty:tracing` | 🔜 Planned | [tracing.md](./tracing.md) |
| `trusty:config` | 🔜 Planned | [config.md](./config.md) |
| `trusty:cli` | 🔜 Planned | [cli.md](./cli.md) |
| `trusty:process` | 🔜 Planned | [process.md](./process.md) |
| `trusty:docker` | 🔜 Planned | [docker.md](./docker.md) |
| `trusty:vault` | 🔜 Planned | [vault.md](./vault.md) |
| `trusty:uuid` | 🔜 Planned | [uuid.md](./uuid.md) |
| `trusty:regex` | 🔜 Planned | [regex.md](./regex.md) |
| `trusty:base64` | 🔜 Planned | [base64.md](./base64.md) |
| `trusty:compress` | 🔜 Planned | [compress.md](./compress.md) |
| `trusty:serialize` | 🔜 Planned | [serialize.md](./serialize.md) |
| `trusty:collections` | 🔜 Planned | [collections.md](./collections.md) |
| `trusty:tensor` | 🔜 Planned | [tensor.md](./tensor.md) |
| `trusty:nn` | 🔜 Planned | [nn.md](./nn.md) |
| `trusty:train` | 🔜 Planned | [train.md](./train.md) |
| `trusty:model` | 🔜 Planned | [model.md](./model.md) |
| `trusty:embed` | 🔜 Planned | [embed.md](./embed.md) |
| `trusty:linalg` | 🔜 Planned | [linalg.md](./linalg.md) |
| `trusty:stats` | 🔜 Planned | [stats.md](./stats.md) |
| `trusty:gpu` | 🔜 Planned | [gpu.md](./gpu.md) |

## Resolution Model

The compiler resolves `trusty:<module>` imports in `crates/trusty-compiler/src/stdlib/mod.rs`.

- If implemented, TRUST injects runtime/type helpers into generated Rust.
- If missing, TRUST emits a `module not yet implemented` comment in generated Rust.

## Implemented Runtime Crates

- `trusty:time` -> std only
- `trusty:math` -> std only
- `trusty:rand` -> `rand`
- `trusty:json` -> `serde`, `serde_derive`, `serde_json`
- `trusty:http` -> `ureq`, `tiny_http`, `serde_json`
