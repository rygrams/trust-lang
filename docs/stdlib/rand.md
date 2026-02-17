# trusty:rand

Random helpers and utilities.

```typescript
import { random, randomInt, randomFloat, bernoulli, weightedIndex, chooseOne, shuffle } from "trusty:rand";
```

## API

- `random(): float64`
- `randomInt(min: int32, max: int32): int32`
- `randomFloat(min: float64, max: float64): float64`
- `bernoulli(p: float64): boolean`
- `weightedIndex(weights: float64[]): int32`
- `chooseOne<T>(items: T[]): T | null`
- `shuffle<T>(items: T[]): T[]`

## Example

```typescript
import { randomInt, chooseOne, shuffle } from "trusty:rand";

function main() {
    console.write(randomInt(1, 6));

    val picked = chooseOne(["a", "b", "c"]);
    console.write(string(picked));

    val out = shuffle([1, 2, 3, 4]);
    console.write(out.join(","));
}
```

## Notes

- Uses Rust crate `rand`.
- `bernoulli(p)` clamps `p` to `[0.0, 1.0]`.
- `weightedIndex(...)` returns `-1` on invalid weights.
