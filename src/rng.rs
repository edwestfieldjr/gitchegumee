//! Seeded RNG, so a run can be replayed from the seed in the log.
//!
//! This is a small PCG32 rather than a `rand` dependency: the game only needs
//! uniform rolls, and the stream must stay stable across dependency upgrades.

use bevy::prelude::*;

const MULTIPLIER: u64 = 6_364_136_223_846_793_005;
const INCREMENT: u64 = 1_442_695_040_888_963_407;

#[derive(Resource, Debug, Clone)]
pub struct GameRng {
    seed: u64,
    state: u64,
}

impl GameRng {
    pub fn new(seed: u64) -> Self {
        let mut rng = Self { seed, state: 0 };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        xorshifted.rotate_right((old >> 59) as u32)
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[low, high)`.
    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    pub fn chance(&mut self, probability: f32) -> bool {
        self.unit() < probability
    }
}

/// Seed for a new run: `DOWNBOUND_SEED` if it is set, otherwise the time.
pub fn choose_seed() -> u64 {
    if let Some(seed) = std::env::var("DOWNBOUND_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
    {
        return seed;
    }
    clock_seed()
}

#[cfg(not(target_arch = "wasm32"))]
fn clock_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1975, |elapsed| elapsed.as_nanos() as u64)
}

/// `SystemTime` panics in a browser, so ask the page for the time.
#[cfg(target_arch = "wasm32")]
fn clock_seed() -> u64 {
    js_sys::Date::now() as u64
}

pub struct RngPlugin;

impl Plugin for RngPlugin {
    fn build(&self, app: &mut App) {
        let rng = GameRng::new(choose_seed());
        info!("RNG seed: {}", rng.seed());
        app.insert_resource(rng);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_gives_same_stream() {
        let mut a = GameRng::new(1975);
        let mut b = GameRng::new(1975);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = GameRng::new(1);
        let mut b = GameRng::new(2);
        assert!((0..8).any(|_| a.next_u32() != b.next_u32()));
    }

    #[test]
    fn unit_stays_in_range() {
        let mut rng = GameRng::new(29);
        for _ in 0..10_000 {
            assert!((0.0..1.0).contains(&rng.unit()));
        }
    }
}
