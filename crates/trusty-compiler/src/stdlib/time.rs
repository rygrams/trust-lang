/// `use` statements injected when `import ... from "trusty:time"` is detected.
pub fn use_statements() -> Vec<&'static str> {
    vec![
        "use std::time::{Instant, Duration, SystemTime as RustSystemTime};",
        "use std::thread::sleep;",
        r#"const TRUST_MILLIS_PER_SECOND: i64 = 1_000;
const TRUST_MILLIS_PER_MINUTE: i64 = 60_000;
const TRUST_MILLIS_PER_HOUR: i64 = 3_600_000;
const TRUST_MILLIS_PER_DAY: i64 = 86_400_000;

fn __trust_days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = year as i64 - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = month as i64;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn __trust_civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

fn __trust_time_from_millis_of_day(millis: i64) -> Time {
    let clamped = millis.clamp(0, TRUST_MILLIS_PER_DAY - 1);
    let hour = (clamped / TRUST_MILLIS_PER_HOUR) as u32;
    let minute = ((clamped % TRUST_MILLIS_PER_HOUR) / TRUST_MILLIS_PER_MINUTE) as u32;
    let second = ((clamped % TRUST_MILLIS_PER_MINUTE) / TRUST_MILLIS_PER_SECOND) as u32;
    let millisecond = (clamped % TRUST_MILLIS_PER_SECOND) as u32;
    Time { hour, minute, second, millisecond }
}

fn __trust_system_time_to_millis(st: RustSystemTime) -> i64 {
    match st.duration_since(RustSystemTime::UNIX_EPOCH) {
        Ok(d) => d.as_millis() as i64,
        Err(e) => -(e.duration().as_millis() as i64),
    }
}

fn __trust_millis_to_system_time(ms: i64) -> RustSystemTime {
    if ms >= 0 {
        RustSystemTime::UNIX_EPOCH + Duration::from_millis(ms as u64)
    } else {
        RustSystemTime::UNIX_EPOCH - Duration::from_millis(ms.unsigned_abs())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

#[allow(non_snake_case)]
impl Date {
    pub fn fromYmd(year: i32, month: i32, day: i32) -> Date {
        let month_u = month.clamp(1, 12) as u32;
        let max_day = Date::daysInMonth(year, month_u as i32) as i32;
        let day_u = day.clamp(1, max_day) as u32;
        Date { year, month: month_u, day: day_u }
    }

    pub fn today() -> Date {
        Date::fromSystemTime(RustSystemTime::now())
    }

    pub fn now() -> Date {
        Date::today()
    }

    pub fn fromSystemTime(st: RustSystemTime) -> Date {
        let millis = __trust_system_time_to_millis(st);
        let days = millis.div_euclid(TRUST_MILLIS_PER_DAY);
        let (year, month, day) = __trust_civil_from_days(days);
        Date { year, month, day }
    }

    pub fn toSystemTime(&self) -> RustSystemTime {
        let days = __trust_days_from_civil(self.year, self.month, self.day);
        let millis = days.saturating_mul(TRUST_MILLIS_PER_DAY);
        __trust_millis_to_system_time(millis)
    }

    pub fn toUnixDays(&self) -> i64 {
        __trust_days_from_civil(self.year, self.month, self.day)
    }

    pub fn dayOfWeek(&self) -> i32 {
        ((self.toUnixDays() + 4).rem_euclid(7)) as i32
    }

    pub fn isLeapYear(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    pub fn daysInMonth(year: i32, month: i32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => if Date::isLeapYear(year) { 29 } else { 28 },
            _ => 30,
        }
    }

    pub fn addDays(&self, days: i32) -> Date {
        let next_days = self.toUnixDays().saturating_add(days as i64);
        let (year, month, day) = __trust_civil_from_days(next_days);
        Date { year, month, day }
    }

    pub fn addMonths(&self, months: i32) -> Date {
        let total_months = (self.year as i64)
            .saturating_mul(12)
            .saturating_add(self.month as i64 - 1)
            .saturating_add(months as i64);
        let new_year_i64 = total_months.div_euclid(12);
        let new_month_i64 = total_months.rem_euclid(12) + 1;
        let new_year = new_year_i64.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        let new_month = new_month_i64 as i32;
        let max_day = Date::daysInMonth(new_year, new_month);
        Date {
            year: new_year,
            month: new_month as u32,
            day: self.day.min(max_day),
        }
    }

    pub fn addYears(&self, years: i32) -> Date {
        self.addMonths(years.saturating_mul(12))
    }

    pub fn subDays(&self, days: i32) -> Date {
        self.addDays(days.saturating_neg())
    }

    pub fn subMonths(&self, months: i32) -> Date {
        self.addMonths(months.saturating_neg())
    }

    pub fn subYears(&self, years: i32) -> Date {
        self.addYears(years.saturating_neg())
    }

    pub fn compare(a: Date, b: Date) -> i32 {
        use std::cmp::Ordering;
        match a.cmp(&b) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    pub fn toString(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    pub fn toIsoString(&self) -> String {
        self.toString()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Time {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub millisecond: u32,
}

#[allow(non_snake_case)]
impl Time {
    pub fn fromHms(hour: i32, minute: i32, second: i32) -> Time {
        Time::fromHmsMilli(hour, minute, second, 0)
    }

    pub fn fromHmsMilli(hour: i32, minute: i32, second: i32, millisecond: i32) -> Time {
        let mut total = (hour as i64).saturating_mul(TRUST_MILLIS_PER_HOUR);
        total = total.saturating_add((minute as i64).saturating_mul(TRUST_MILLIS_PER_MINUTE));
        total = total.saturating_add((second as i64).saturating_mul(TRUST_MILLIS_PER_SECOND));
        total = total.saturating_add(millisecond as i64);
        let normalized = total.rem_euclid(TRUST_MILLIS_PER_DAY);
        __trust_time_from_millis_of_day(normalized)
    }

    pub fn midnight() -> Time {
        Time { hour: 0, minute: 0, second: 0, millisecond: 0 }
    }

    pub fn now() -> Time {
        Time::fromSystemTime(RustSystemTime::now())
    }

    pub fn fromSystemTime(st: RustSystemTime) -> Time {
        let millis = __trust_system_time_to_millis(st);
        let day_millis = millis.rem_euclid(TRUST_MILLIS_PER_DAY);
        __trust_time_from_millis_of_day(day_millis)
    }

    pub fn toMillisOfDay(&self) -> i64 {
        (self.hour as i64).saturating_mul(TRUST_MILLIS_PER_HOUR)
            .saturating_add((self.minute as i64).saturating_mul(TRUST_MILLIS_PER_MINUTE))
            .saturating_add((self.second as i64).saturating_mul(TRUST_MILLIS_PER_SECOND))
            .saturating_add(self.millisecond as i64)
    }

    pub fn addSeconds(&self, seconds: i32) -> Time {
        let delta = (seconds as i64).saturating_mul(TRUST_MILLIS_PER_SECOND);
        let normalized = self.toMillisOfDay().saturating_add(delta).rem_euclid(TRUST_MILLIS_PER_DAY);
        __trust_time_from_millis_of_day(normalized)
    }

    pub fn addMinutes(&self, minutes: i32) -> Time {
        self.addSeconds(minutes.saturating_mul(60))
    }

    pub fn addHours(&self, hours: i32) -> Time {
        self.addMinutes(hours.saturating_mul(60))
    }

    pub fn subSeconds(&self, seconds: i32) -> Time {
        self.addSeconds(seconds.saturating_neg())
    }

    pub fn subMinutes(&self, minutes: i32) -> Time {
        self.addMinutes(minutes.saturating_neg())
    }

    pub fn subHours(&self, hours: i32) -> Time {
        self.addHours(hours.saturating_neg())
    }

    pub fn compare(a: Time, b: Time) -> i32 {
        use std::cmp::Ordering;
        match a.cmp(&b) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    pub fn toString(&self) -> String {
        format!("{:02}:{:02}:{:02}.{:03}", self.hour, self.minute, self.second, self.millisecond)
    }

    pub fn toIsoString(&self) -> String {
        self.toString()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct DateTime {
    pub date: Date,
    pub time: Time,
}

#[allow(non_snake_case)]
impl DateTime {
    pub fn fromParts(date: Date, time: Time) -> DateTime {
        DateTime { date, time }
    }

    pub fn now() -> DateTime {
        DateTime::fromSystemTime(RustSystemTime::now())
    }

    pub fn fromSystemTime(st: RustSystemTime) -> DateTime {
        DateTime { date: Date::fromSystemTime(st), time: Time::fromSystemTime(st) }
    }

    pub fn toSystemTime(&self) -> RustSystemTime {
        __trust_millis_to_system_time(self.toTimestampMillis())
    }

    pub fn fromTimestampMillis(ms: i64) -> DateTime {
        let days = ms.div_euclid(TRUST_MILLIS_PER_DAY);
        let day_millis = ms.rem_euclid(TRUST_MILLIS_PER_DAY);
        let (year, month, day) = __trust_civil_from_days(days);
        DateTime {
            date: Date { year, month, day },
            time: __trust_time_from_millis_of_day(day_millis),
        }
    }

    pub fn toTimestampMillis(&self) -> i64 {
        let days = self.date.toUnixDays();
        days.saturating_mul(TRUST_MILLIS_PER_DAY)
            .saturating_add(self.time.toMillisOfDay())
    }

    pub fn addSeconds(&self, seconds: i32) -> DateTime {
        let delta = (seconds as i64).saturating_mul(TRUST_MILLIS_PER_SECOND);
        DateTime::fromTimestampMillis(self.toTimestampMillis().saturating_add(delta))
    }

    pub fn addMinutes(&self, minutes: i32) -> DateTime {
        self.addSeconds(minutes.saturating_mul(60))
    }

    pub fn addHours(&self, hours: i32) -> DateTime {
        self.addMinutes(hours.saturating_mul(60))
    }

    pub fn addDays(&self, days: i32) -> DateTime {
        let delta = (days as i64).saturating_mul(TRUST_MILLIS_PER_DAY);
        DateTime::fromTimestampMillis(self.toTimestampMillis().saturating_add(delta))
    }

    pub fn addMonths(&self, months: i32) -> DateTime {
        DateTime {
            date: self.date.addMonths(months),
            time: self.time,
        }
    }

    pub fn addYears(&self, years: i32) -> DateTime {
        self.addMonths(years.saturating_mul(12))
    }

    pub fn subSeconds(&self, seconds: i32) -> DateTime {
        self.addSeconds(seconds.saturating_neg())
    }

    pub fn subMinutes(&self, minutes: i32) -> DateTime {
        self.addMinutes(minutes.saturating_neg())
    }

    pub fn subHours(&self, hours: i32) -> DateTime {
        self.addHours(hours.saturating_neg())
    }

    pub fn subDays(&self, days: i32) -> DateTime {
        self.addDays(days.saturating_neg())
    }

    pub fn subMonths(&self, months: i32) -> DateTime {
        self.addMonths(months.saturating_neg())
    }

    pub fn subYears(&self, years: i32) -> DateTime {
        self.addYears(years.saturating_neg())
    }

    pub fn startOfDay(&self) -> DateTime {
        DateTime { date: self.date, time: Time::midnight() }
    }

    pub fn endOfDay(&self) -> DateTime {
        DateTime {
            date: self.date,
            time: Time::fromHmsMilli(23, 59, 59, 999),
        }
    }

    pub fn compare(a: DateTime, b: DateTime) -> i32 {
        use std::cmp::Ordering;
        match a.cmp(&b) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    pub fn toString(&self) -> String {
        format!("{}T{}Z", self.date.toString(), self.time.toString())
    }

    pub fn toIsoString(&self) -> String {
        self.toString()
    }
}

pub type SystemTime = DateTime;

#[allow(non_snake_case)]
fn compare(a: DateTime, b: DateTime) -> i32 {
    DateTime::compare(a, b)
}

#[allow(non_snake_case)]
fn addSeconds(dateTime: DateTime, seconds: i32) -> DateTime {
    dateTime.addSeconds(seconds)
}

#[allow(non_snake_case)]
fn addMinutes(dateTime: DateTime, minutes: i32) -> DateTime {
    dateTime.addMinutes(minutes)
}

#[allow(non_snake_case)]
fn addDays(dateTime: DateTime, days: i32) -> DateTime {
    dateTime.addDays(days)
}

#[allow(non_snake_case)]
fn addMonths(dateTime: DateTime, months: i32) -> DateTime {
    dateTime.addMonths(months)
}

#[allow(non_snake_case)]
fn addYears(dateTime: DateTime, years: i32) -> DateTime {
    dateTime.addYears(years)
}

#[allow(non_snake_case)]
fn subSeconds(dateTime: DateTime, seconds: i32) -> DateTime {
    dateTime.subSeconds(seconds)
}

#[allow(non_snake_case)]
fn subMinutes(dateTime: DateTime, minutes: i32) -> DateTime {
    dateTime.subMinutes(minutes)
}

#[allow(non_snake_case)]
fn subDays(dateTime: DateTime, days: i32) -> DateTime {
    dateTime.subDays(days)
}

#[allow(non_snake_case)]
fn subMonths(dateTime: DateTime, months: i32) -> DateTime {
    dateTime.subMonths(months)
}

#[allow(non_snake_case)]
fn subYears(dateTime: DateTime, years: i32) -> DateTime {
    dateTime.subYears(years)
}"#,
    ]
}

/// No external crates needed — everything is in `std`.
pub fn required_crates() -> Vec<(&'static str, &'static str)> {
    vec![]
}

/// Maps TRUST `Duration` static constructors to Rust equivalents.
///
/// | TRUST                  | Rust                          |
/// |------------------------|-------------------------------|
/// | `Duration.millis(n)`   | `Duration::from_millis(n)`    |
/// | `Duration.seconds(n)`  | `Duration::from_secs(n)`      |
/// | `Duration.minutes(n)`  | `Duration::from_secs(n * 60)` |
/// | `Duration.nanos(n)`    | `Duration::from_nanos(n)`     |
/// | `Duration.micros(n)`   | `Duration::from_micros(n)`    |
pub fn map_duration_constructor(method: &str, arg: &str) -> Option<String> {
    match method {
        "millis" => Some(format!("Duration::from_millis(({}) as u64)", arg)),
        "seconds" | "secs" => Some(format!("Duration::from_secs(({}) as u64)", arg)),
        "minutes" => Some(format!("Duration::from_secs((({}) as u64) * 60)", arg)),
        "nanos" => Some(format!("Duration::from_nanos(({}) as u64)", arg)),
        "micros" => Some(format!("Duration::from_micros(({}) as u64)", arg)),
        _ => None,
    }
}

/// Maps TRUST duration/instant instance method names to Rust method names.
///
/// | TRUST              | Rust            |
/// |--------------------|-----------------|
/// | `.asMillis()`      | `.as_millis()`  |
/// | `.asSeconds()`     | `.as_secs()`    |
/// | `.asNanos()`       | `.as_nanos()`   |
/// | `.asMicros()`      | `.as_micros()`  |
/// | `.asSecsFloat()`   | `.as_secs_f64()`|
pub fn map_instance_method(method: &str) -> Option<&'static str> {
    match method {
        "asMillis" => Some("as_millis"),
        "asSeconds" | "asSecs" => Some("as_secs"),
        "asNanos" => Some("as_nanos"),
        "asMicros" => Some("as_micros"),
        "asSecsFloat" => Some("as_secs_f64"),
        _ => None,
    }
}
