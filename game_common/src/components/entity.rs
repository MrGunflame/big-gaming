use std::time::{Duration, Instant};

use bevy_ecs::component::Component;
use glam::Vec3;

/// An [`Entity`] that exists within the game world.
///
/// This only includes entities that exist within the world, i.e. excludes components like cameras,
/// markers, UI, etc..
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct WorldObject;

/// A [`WorldObject`] of low importance that should not be saved between runs.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct TemporaryObject;

/// A [`WorldObject`] of high importance that should be saved between runs.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct PersistentObject;

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct EntityName(String);

impl EntityName {
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for EntityName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Copy, Clone, Debug, Component)]
pub struct InterpolateTranslation {
    pub src: Vec3,
    pub dst: Vec3,
    pub start: Instant,
    pub end: Instant,
}

impl InterpolateTranslation {
    pub fn get(&self, now: Instant) -> Vec3 {
        let d1 = self.end - self.start;
        let d2 = now - self.start;

        let s = d2.as_secs_f64() / d1.as_secs_f64();

        Vec3::lerp(self.src, self.dst, f32::clamp(s as f32, 0.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use glam::Vec3;

    use super::InterpolateTranslation;

    #[test]
    fn interpolate_translation() {
        let now = Instant::now();

        let lerp = InterpolateTranslation {
            src: Vec3::splat(0.0),
            dst: Vec3::splat(1.0),
            start: now,
            end: now + Duration::from_secs(1),
        };

        assert_eq!(lerp.get(now), Vec3::splat(0.0));
        assert_eq!(lerp.get(now + Duration::from_millis(500)), Vec3::splat(0.5));
        assert_eq!(lerp.get(now + Duration::from_secs(1)), Vec3::splat(1.0));
    }
}
