# trusty:http

HTTP client and server/router module.

```typescript
import { fetch, fetchWith, requestOptions, HttpRequestOptions, HttpServer } from "trusty:http";
```

## Client API

- `requestOptions(): HttpRequestOptions`
- `fetch(url: string): HttpResponse`
- `fetchWith(url: string, options: HttpRequestOptions): HttpResponse`

`HttpRequestOptions` fields:
- `method: string` (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`)
- `headers: Map<string, string>`
- `body: string`
- `timeoutMs: int32`

`HttpResponse` fields and methods:
- `status: int32`
- `ok: boolean`
- `body: string`
- `headers: Map<string, string>`
- `error: string`
- `text(): string`
- `json(): JSONValue` (fallback `null`)
- `jsonAs<T>(): T | null`
- `header(name: string): string`

## Server/Router API

- `HttpServer.create()`
- `app.get(path, handler)`
- `app.post(path, handler)`
- `app.put(path, handler)`
- `app.delete(path, handler)`
- `app.addMiddleware(fn(req) => req)`
- `app.listen(port): boolean`
- `app.listenOn(bindAddress): boolean` (example: `"127.0.0.1:8081"`)
- `app.lastError(): string`

Handler signature:
- `function(req, res) { ... }`

`Request`:
- `method`, `path`, `query`, `headers`, `body`, `params`
- `text()`, `json()`, `jsonAs<T>()`, `header(name)`

`Params`:
- `getOr(key, fallback)`

`Response`:
- `status(code)`
- `header(name, value)`
- `send(body)`
- `json(jsonString)`
- `jsonValue(jsonValue)`

## Example

```typescript
import { HttpServer } from "trusty:http";
import { toJSON } from "trusty:json";

function main() {
    val app = HttpServer.create();

    app.get("/users/:id", function(req, res) {
        val id = req.params.getOr("id", "unknown");
        res.status(200).json(toJSON({ ok: true, id: id }));
    });

    val started = app.listenOn("127.0.0.1:8081");
    if (!started) {
        console.write(app.lastError());
    }
}
```

## Notes

- Uses `ureq` for client and `tiny_http` for server runtime.
- No `unwrap` is required in TRUST user code.
- If `listen(...)` returns `false`, check `lastError()`; common case is port already in use.
