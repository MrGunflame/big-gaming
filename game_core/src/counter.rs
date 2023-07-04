use std::time::{Duration, Instant};

/// Updates per second.
#[derive(Clone, Debug)]
pub struct UpdateCounter {
    ups: f32,
    prev: Instant,
    /// Update period end
    end: Instant,
}

impl UpdateCounter {
    pub fn new() -> Self {
        let now = Instant::now();

        Self {
            ups: 60.0,
            prev: now,
            end: now + Duration::from_secs(1),
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();

        if now >= self.end {
            //self.prev = now;
            self.end = now + Duration::from_secs(1);
        }

        let frame_time = now - self.prev;
        self.prev = now;
        let ups = Duration::from_secs(1).as_secs_f32() / frame_time.as_secs_f32();

        self.ups = (self.ups * 0.8) + (ups * 0.2);
    }

    pub fn ups(&self) -> f32 {
        self.ups
    }
}
