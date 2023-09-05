//! Time plugin for advancing frame times
//!
//! Not to be confused with in-game time.

use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct Time {
    startup: Instant,
    speed: f64,
    delta: Duration,
    last_update: Instant,
}

impl Time {
    pub fn new() -> Self {
        let now = Instant::now();

        Self {
            startup: now,
            speed: 1.0,
            delta: Duration::ZERO,
            last_update: now,
        }
    }

    pub fn delta(&self) -> Duration {
        self.delta
    }

    pub fn last_update(&self) -> Instant {
        self.last_update
    }

    pub fn update(&mut self) {
        let now = Instant::now();

        self.delta = now - self.last_update;
        self.last_update = now;
    }
}
