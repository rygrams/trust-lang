# trusty:time

Standard library module for time measurement, durations, and sleeping.

```typescript
import { Instant, Duration, SystemTime, Date, Time, DateTime, sleep, compare, addDays, addMonths, addYears, addMinutes, addSeconds, subDays, subMonths, subYears, subMinutes, subSeconds } from "trusty:time";
```

> Backed by `std::time` and `std::thread::sleep` — no external crate required.
> All symbols are zero-cost: unused ones are stripped at compile time.

---

## `Instant`

A monotonic clock snapshot. Used to measure elapsed time.

### Static methods

| TRUST | Rust generated | Description |
|---|---|---|
| `Instant.now()` | `Instant::now()` | Capture the current moment |

### Instance methods

| TRUST | Rust generated | Returns | Description |
|---|---|---|---|
| `instant.elapsed()` | `instant.elapsed()` | `Duration` | Time elapsed since the snapshot |

**Example**

```typescript
val start = Instant.now();
// ... work ...
val elapsed = start.elapsed();
console.write(`Done in ${elapsed.asMillis()} ms`);
```

---

## `Duration`

A span of time. Created via static constructors; read via instance methods.

### Static constructors

| TRUST | Rust generated | Description |
|---|---|---|
| `Duration.nanos(n)` | `Duration::from_nanos((n) as u64)` | From nanoseconds |
| `Duration.micros(n)` | `Duration::from_micros((n) as u64)` | From microseconds |
| `Duration.millis(n)` | `Duration::from_millis((n) as u64)` | From milliseconds |
| `Duration.seconds(n)` | `Duration::from_secs((n) as u64)` | From seconds |
| `Duration.secs(n)` | `Duration::from_secs((n) as u64)` | Alias for `seconds` |
| `Duration.minutes(n)` | `Duration::from_secs(((n) as u64) * 60)` | From minutes |

### Instance methods (getters)

| TRUST | Rust generated | Returns | Description |
|---|---|---|---|
| `d.asNanos()` | `d.as_nanos()` | `u128` | Total nanoseconds |
| `d.asMicros()` | `d.as_micros()` | `u128` | Total microseconds |
| `d.asMillis()` | `d.as_millis()` | `u128` | Total milliseconds |
| `d.asSeconds()` | `d.as_secs()` | `u64` | Total whole seconds |
| `d.asSecs()` | `d.as_secs()` | `u64` | Alias for `asSeconds` |
| `d.asSecsFloat()` | `d.as_secs_f64()` | `f64` | Seconds as floating-point |

**Example**

```typescript
val d = Duration.minutes(2);
console.write(`${d.asSeconds()} s`);   // 120 s
console.write(`${d.asMillis()} ms`);   // 120000 ms
```

---

## `sleep`

Blocks the current thread for the given duration.

```typescript
sleep(duration: Duration): void
```

| TRUST | Rust generated |
|---|---|
| `sleep(Duration.millis(500))` | `sleep(Duration::from_millis((500) as u64))` |

**Example**

```typescript
console.write("waiting...");
sleep(Duration.seconds(1));
console.write("done");
```

---

## `DateTime` global helpers (light date-fns style)

Helpers for basic date arithmetic and comparison on `DateTime`.
`SystemTime` remains an alias of `DateTime` (UTC-oriented), but these functions are typed for `DateTime`.

| TRUST | Rust generated | Description |
|---|---|---|
| `compare(a, b)` | `compare(a, b)` | `-1`, `0`, `1` based on ordering |
| `addSeconds(dateTime, n)` | `addSeconds(dateTime, n)` | Add seconds |
| `addMinutes(dateTime, n)` | `addMinutes(dateTime, n)` | Add minutes |
| `addDays(dateTime, n)` | `addDays(dateTime, n)` | Add days |
| `addMonths(dateTime, n)` | `addMonths(dateTime, n)` | Add months |
| `addYears(dateTime, n)` | `addYears(dateTime, n)` | Add years |
| `subSeconds(dateTime, n)` | `subSeconds(dateTime, n)` | Subtract seconds |
| `subMinutes(dateTime, n)` | `subMinutes(dateTime, n)` | Subtract minutes |
| `subDays(dateTime, n)` | `subDays(dateTime, n)` | Subtract days |
| `subMonths(dateTime, n)` | `subMonths(dateTime, n)` | Subtract months |
| `subYears(dateTime, n)` | `subYears(dateTime, n)` | Subtract years |

**Example**

```typescript
import { DateTime, compare, addDays, subMinutes } from "trusty:time";

val now = DateTime.now();
val later = addDays(now, 2);
val before = subMinutes(now, 30);
console.write(compare(before, later)); // -1
```

---

## `Date`

Calendar date helpers.

| TRUST | Description |
|---|---|
| `Date.fromYmd(y, m, d)` | Create date (clamped to valid month/day) |
| `Date.today()` / `Date.now()` | Current UTC date |
| `Date.isLeapYear(y)` | Leap year check |
| `Date.daysInMonth(y, m)` | Days in month |
| `d.addDays(n)` / `d.subDays(n)` | Add/subtract days |
| `d.addMonths(n)` / `d.subMonths(n)` | Add/subtract months |
| `d.addYears(n)` / `d.subYears(n)` | Add/subtract years |
| `Date.compare(a, b)` | `-1`, `0`, `1` |
| `d.toString()` | `YYYY-MM-DD` |
| `d.toIsoString()` | ISO date string (`YYYY-MM-DD`) |

## `Time`

Clock time helpers.

| TRUST | Description |
|---|---|
| `Time.fromHms(h, m, s)` | Create time |
| `Time.fromHmsMilli(h, m, s, ms)` | Create time with milliseconds |
| `Time.midnight()` / `Time.now()` | Common constructors |
| `t.addHours(n)` / `t.addMinutes(n)` / `t.addSeconds(n)` | Add |
| `t.subHours(n)` / `t.subMinutes(n)` / `t.subSeconds(n)` | Sub |
| `Time.compare(a, b)` | `-1`, `0`, `1` |
| `t.toString()` | `HH:MM:SS.mmm` |
| `t.toIsoString()` | ISO time string (`HH:MM:SS.mmm`) |

## `DateTime`

Date+time helpers.

| TRUST | Description |
|---|---|
| `DateTime.fromParts(date, time)` | Build from `Date` + `Time` |
| `DateTime.now()` | Current UTC date-time |
| `DateTime.fromTimestampMillis(ms)` | Build from unix ms |
| `dt.toTimestampMillis()` | Convert to unix ms |
| `dt.addDays/minutes/hours/seconds(n)` | Add |
| `dt.subDays/minutes/hours/seconds(n)` | Sub |
| `dt.addMonths(n)` / `dt.subMonths(n)` | Add/subtract months |
| `dt.addYears(n)` / `dt.subYears(n)` | Add/subtract years |
| `dt.startOfDay()` / `dt.endOfDay()` | Day boundaries |
| `DateTime.compare(a, b)` | `-1`, `0`, `1` |
| `dt.toString()` | `YYYY-MM-DDTHH:MM:SS.mmmZ` |
| `dt.toIsoString()` | ISO datetime string (`YYYY-MM-DDTHH:MM:SS.mmmZ`) |

---

## Full example

```typescript
import { Instant, Duration, SystemTime, Date, Time, DateTime, sleep, compare, addDays } from "trusty:time";

function main() {
    // Benchmark a block
    val start = Instant.now();

    sleep(Duration.millis(200));

    val elapsed = start.elapsed();
    console.write(`Elapsed: ${elapsed.asMillis()} ms`);
    console.write(`Elapsed (float): ${elapsed.asSecsFloat()} s`);

    // Duration conversions
    val one_hour = Duration.minutes(60);
    console.write(`1 hour = ${one_hour.asSeconds()} s`);
    console.write(`1 hour = ${one_hour.asMillis()} ms`);

    // SystemTime helpers
    val now = SystemTime.now();
    val next_week = addDays(now, 7);
    console.write(`compare(now, next_week) = ${compare(now, next_week)}`);

    // Date / Time / DateTime helpers
    val d = Date.fromYmd(2026, 2, 15).addDays(1);
    val t = Time.fromHmsMilli(10, 30, 0, 0).subMinutes(15);
    val dt = DateTime.fromParts(d, t).startOfDay();
    console.write(d.toIsoString());
    console.write(t.toIsoString());
    console.write(dt.toIsoString());
}
```

Generated Rust:

```rust
use std::time::{Instant, Duration, SystemTime as RustSystemTime};
use std::thread::sleep;

fn main() -> () {
    let start = Instant::now();
    sleep(Duration::from_millis((200) as u64));
    let elapsed = start.elapsed();
    println!("{}", format!("Elapsed: {} ms", elapsed.as_millis()));
    println!("{}", format!("Elapsed (float): {} s", elapsed.as_secs_f64()));
    let one_hour = Duration::from_secs(((60) as u64) * 60);
    println!("{}", format!("1 hour = {} s", one_hour.as_secs()));
    println!("{}", format!("1 hour = {} ms", one_hour.as_millis()));
    let now = SystemTime::now();
    let next_week = addDays(now, 7);
    println!("{}", format!("compare(now, next_week) = {}", compare(now, next_week)));
}
```

---

## Notes

- `Duration` constructors take integers. Pass an `int32` variable and the transpiler adds `as u64` automatically.
- `asMillis()` / `asNanos()` return `u128` — they can overflow when used in arithmetic with `int32`. Cast explicitly if needed: `int32(elapsed.asMillis())`.
- `sleep` is synchronous and blocks the OS thread. For async code, use `trusty:async` (planned).
