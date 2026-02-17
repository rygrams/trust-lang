# trusty:json

JSON parse and serialization helpers.

```typescript
import { parseToJSON, stringify, toJSON, fromJSON } from "trusty:json";
```

## API

- `parseToJSON(json: string): JSONValue`
- `stringify(value): string`
- `toJSON(value): string`
- `fromJSON<T>(json: string): T`

## Example

```typescript
import { toJSON, fromJSON, parseToJSON } from "trusty:json";

struct User {
    id: int32;
    name: string;
}

function main() {
    val u: User = User({ id: 1, name: "Ryan" });
    val raw = toJSON(u);
    val typed: User = fromJSON(raw);

    val obj = parseToJSON("{\"ok\":true}");

    console.write(raw);
    console.write(typed.name);
    console.write(toJSON(obj));
}
```

## Notes

- Uses `serde`, `serde_derive`, `serde_json`.
- TRUST `struct` types derive serde traits automatically when `trusty:json` is imported.
- `parseToJSON` returns `null` (`Value::Null`) if parsing fails.
