use std::time::Instant;

use bevy_ecs::prelude::{Component, Entity};
use bevy_ecs::system::{Commands, Query, Res};
use game_common::components::actor::ActorProperties;
use game_common::components::transform::Transform;
use game_core::time::Time;
use glam::{Quat, Vec3};

use crate::utils::extract_actor_rotation;

#[derive(Copy, Clone, Debug, Component)]
pub struct InterpolateTranslation {
    pub src: Vec3,
    pub dst: Vec3,
    pub start: Instant,
    pub end: Instant,
}

impl InterpolateTranslation {
    fn get(&self, now: Instant) -> Vec3 {
        let d1 = self.end - self.start;
        let d2 = now - self.start;

        let s = d2.as_secs_f64() / d1.as_secs_f64();

        Vec3::lerp(self.src, self.dst, f32::clamp(s as f32, 0.0, 1.0))
    }
}

#[derive(Copy, Clone, Debug, Component)]
pub struct InterpolateRotation {
    pub src: Quat,
    pub dst: Quat,
    pub start: Instant,
    pub end: Instant,
}

impl InterpolateRotation {
    fn get(&self, now: Instant) -> Quat {
        let d1 = self.end - self.start;
        let d2 = now - self.start;

        let s = d2.as_secs_f64() / d1.as_secs_f64();

        Quat::slerp(self.src, self.dst, f32::clamp(s as f32, 0.0, 1.0))
    }
}

pub fn interpolate_translation(
    time: Res<Time>,
    mut commands: Commands,
    mut entities: Query<(Entity, &mut Transform, &InterpolateTranslation)>,
) {
    let now = time.last_update();

    for (entity, mut transform, interpolate) in &mut entities {
        let now = now - (interpolate.end - interpolate.start);

        transform.translation = interpolate.get(now);

        if now >= interpolate.end {
            commands.entity(entity).remove::<InterpolateTranslation>();
        }
    }
}

pub fn interpolate_rotation(
    time: Res<Time>,
    mut commands: Commands,
    mut entities: Query<(
        Entity,
        &mut Transform,
        Option<&mut ActorProperties>,
        &InterpolateRotation,
    )>,
) {
    let now = time.last_update();

    for (entity, mut transform, props, interpolate) in &mut entities {
        let now = now - (interpolate.end - interpolate.start);

        if let Some(mut props) = props {
            props.rotation = interpolate.get(now);
            transform.rotation = extract_actor_rotation(props.rotation);
        } else {
            transform.rotation = interpolate.get(now);
        }

        if now >= interpolate.end {
            commands.entity(entity).remove::<InterpolateRotation>();
        }
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
