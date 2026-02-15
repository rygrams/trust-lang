/// `use` statements injected when `import ... from "trusty:rand"` is detected.
pub fn use_statements() -> Vec<&'static str> {
    vec![r#"use rand::Rng;
use rand::distributions::{Bernoulli, Distribution, WeightedIndex};
use rand::seq::SliceRandom;

#[allow(non_snake_case)]
pub fn random() -> f64 {
    let mut rng = rand::thread_rng();
    rng.gen::<f64>()
}

#[allow(non_snake_case)]
pub fn randomInt(min: i32, max: i32) -> i32 {
    let mut rng = rand::thread_rng();
    if min <= max {
        rng.gen_range(min..=max)
    } else {
        rng.gen_range(max..=min)
    }
}

#[allow(non_snake_case)]
pub fn randomFloat(min: f64, max: f64) -> f64 {
    let mut rng = rand::thread_rng();
    let lo = min.min(max);
    let hi = min.max(max);
    if (hi - lo).abs() < f64::EPSILON {
        lo
    } else {
        rng.gen_range(lo..hi)
    }
}

#[allow(non_snake_case)]
pub fn bernoulli(p: f64) -> bool {
    let mut rng = rand::thread_rng();
    let prob = p.clamp(0.0, 1.0);
    Bernoulli::new(prob).map(|d| d.sample(&mut rng)).unwrap_or(false)
}

#[allow(non_snake_case)]
pub fn weightedIndex(weights: Vec<f64>) -> i32 {
    let mut rng = rand::thread_rng();
    match WeightedIndex::new(weights) {
        Ok(dist) => dist.sample(&mut rng) as i32,
        Err(_) => -1,
    }
}

#[allow(non_snake_case)]
pub fn chooseOne<T: Clone>(items: Vec<T>) -> Option<T> {
    let mut rng = rand::thread_rng();
    items.choose(&mut rng).cloned()
}

#[allow(non_snake_case)]
pub fn shuffle<T: Clone>(items: Vec<T>) -> Vec<T> {
    let mut rng = rand::thread_rng();
    let mut out = items.clone();
    out.shuffle(&mut rng);
    out
}"#]
}

/// External crate needed.
pub fn required_crates() -> Vec<(&'static str, &'static str)> {
    vec![("rand", "0.8")]
}
