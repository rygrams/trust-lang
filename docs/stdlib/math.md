# trusty:math

Math helpers (`sqrt`, `pow`, `log`, `abs`, trig) and constants.

```typescript
import { PI, E, sqrt, pow, log, abs, min, max, clamp, sin, cos, tan, asin, acos, atan } from "trusty:math";
```

## API

- `PI: float64`
- `E: float64`
- `sqrt(x): float64`
- `pow(base, exp): float64`
- `log(value): float64`
- `log(value, base): float64`
- `abs(x): same numeric type`
- `min(a, b): same type`
- `max(a, b): same type`
- `clamp(x, lo, hi): same type`
- `sin/cos/tan(x): float64`
- `asin/acos/atan(x): float64`

## Example

```typescript
import { PI, sqrt, pow, log, clamp } from "trusty:math";

function main() {
    val a = sqrt(9);
    val b = pow(2, 8);
    val c = log(8, 2);
    val d = clamp(120, 0, 100);

    console.write(PI);
    console.write(a);
    console.write(b);
    console.write(c);
    console.write(d);
}
```

## Notes

- Backed by Rust `std` only (no external crate).
- `log(x, base)` is supported by transpiler and maps to an internal `log_base(...)` helper.
