/// `use` statements injected when `import ... from "trusty:math"` is detected.
pub fn use_statements() -> Vec<&'static str> {
    vec![r#"pub const PI: f64 = std::f64::consts::PI;
pub const E: f64 = std::f64::consts::E;

pub trait __TrustMathAbs {
    fn __trust_abs(self) -> Self;
}

impl __TrustMathAbs for i8 {
    fn __trust_abs(self) -> Self { self.abs() }
}
impl __TrustMathAbs for i16 {
    fn __trust_abs(self) -> Self { self.abs() }
}
impl __TrustMathAbs for i32 {
    fn __trust_abs(self) -> Self { self.abs() }
}
impl __TrustMathAbs for i64 {
    fn __trust_abs(self) -> Self { self.abs() }
}
impl __TrustMathAbs for isize {
    fn __trust_abs(self) -> Self { self.abs() }
}
impl __TrustMathAbs for f32 {
    fn __trust_abs(self) -> Self { self.abs() }
}
impl __TrustMathAbs for f64 {
    fn __trust_abs(self) -> Self { self.abs() }
}

#[allow(non_snake_case)]
pub fn sqrt<T: Into<f64>>(x: T) -> f64 {
    x.into().sqrt()
}

#[allow(non_snake_case)]
pub fn pow<A: Into<f64>, B: Into<f64>>(base: A, exp: B) -> f64 {
    base.into().powf(exp.into())
}

#[allow(non_snake_case)]
pub fn log<T: Into<f64>>(value: T) -> f64 {
    value.into().ln()
}

#[allow(non_snake_case)]
pub fn log_base<V: Into<f64>, B: Into<f64>>(value: V, base: B) -> f64 {
    value.into().log(base.into())
}

#[allow(non_snake_case)]
pub fn abs<T: __TrustMathAbs>(x: T) -> T {
    x.__trust_abs()
}

#[allow(non_snake_case)]
pub fn min<T: PartialOrd + Copy>(a: T, b: T) -> T {
    if a <= b { a } else { b }
}

#[allow(non_snake_case)]
pub fn max<T: PartialOrd + Copy>(a: T, b: T) -> T {
    if a >= b { a } else { b }
}

#[allow(non_snake_case)]
pub fn clamp<T: PartialOrd + Copy>(x: T, lo: T, hi: T) -> T {
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}

#[allow(non_snake_case)]
pub fn sin<T: Into<f64>>(x: T) -> f64 {
    x.into().sin()
}

#[allow(non_snake_case)]
pub fn cos<T: Into<f64>>(x: T) -> f64 {
    x.into().cos()
}

#[allow(non_snake_case)]
pub fn tan<T: Into<f64>>(x: T) -> f64 {
    x.into().tan()
}

#[allow(non_snake_case)]
pub fn asin<T: Into<f64>>(x: T) -> f64 {
    x.into().asin()
}

#[allow(non_snake_case)]
pub fn acos<T: Into<f64>>(x: T) -> f64 {
    x.into().acos()
}

#[allow(non_snake_case)]
pub fn atan<T: Into<f64>>(x: T) -> f64 {
    x.into().atan()
}"#]
}

/// No external crates needed — everything is in `std`.
pub fn required_crates() -> Vec<(&'static str, &'static str)> {
    vec![]
}
