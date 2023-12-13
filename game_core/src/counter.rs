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

impl Default for UpdateCounter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct Interval {
    last_update: Instant,
    /// The uniform timestep duration of a control frame.
    timestep: Duration,
}

impl Interval {
    pub fn new(timestep: Duration) -> Self {
        Self {
            last_update: Instant::now(),
            timestep,
        }
    }

    pub fn is_ready(&mut self, now: Instant) -> bool {
        let elapsed = now - self.last_update;

        if elapsed >= self.timestep {
            self.last_update += self.timestep;
            true
        } else {
            false
        }
    }

    /// Sleep until the next tick.
    pub async fn wait(&mut self, now: Instant) {
        // The timer may already be delayed which means we need
        // to yield immediately.
        let elapsed = now - self.last_update;
        if elapsed >= self.timestep {
            self.last_update += self.timestep;
            return;
        }

        // FIXME: This will likely break terribly on windows for <15ms sleep
        // times. We should probably timeBeginPeriod/timeBeginEndPeriod and spin
        // with PAUSEs for small periods.
        // Linux timers are accurate enough (~50us) that we don't really have to
        // bother with it.
        let duration = self.timestep - elapsed;
        tokio::time::sleep(duration).await;
        self.last_update += self.timestep;
    }
}

impl IntervalImpl for Interval {
    fn is_ready(&mut self, now: Instant) -> bool {
        Self::is_ready(self, now)
    }
}

pub trait IntervalImpl {
    fn is_ready(&mut self, now: Instant) -> bool;
}

pub struct ManualInterval {
    /// Should the next call yield ready?
    is_ready: bool,
}

impl ManualInterval {
    pub fn new() -> Self {
        Self { is_ready: false }
    }

    pub fn set_ready(&mut self) {
        self.is_ready = true;
    }
}

impl IntervalImpl for ManualInterval {
    fn is_ready(&mut self, _now: Instant) -> bool {
        let is_ready = self.is_ready;
        self.is_ready ^= true;
        is_ready
    }
}

impl Default for ManualInterval {
    fn default() -> Self {
        Self::new()
    }
}
