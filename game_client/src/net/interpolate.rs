use std::time::Instant;

use bevy_ecs::prelude::{Component, Entity};
use bevy_ecs::system::{Commands, Query, Res};
use game_common::components::actor::ActorProperties;
use game_common::components::transform::Transform;
use game_common::world::control_frame::ControlFrame;
use game_core::time::Time;
use glam::{Quat, Vec3};

use crate::utils::extract_actor_rotation;

use super::ServerConnection;

#[derive(Copy, Clone, Debug, Component)]
pub struct InterpolateTranslation {
    pub src: Vec3,
    pub dst: Vec3,
    pub start: ControlFrame,
    pub end: ControlFrame,
}

impl InterpolateTranslation {
    fn get(&self, now: ControlFrame) -> Vec3 {
        let d1 = self.end - self.start;
        let d2 = now - self.start;

        let s = d2.0 as f32 / d1.0 as f32;

        dbg!(s);

        Vec3::lerp(self.src, self.dst, f32::clamp(s, 0.0, 1.0))
    }
}

#[derive(Copy, Clone, Debug, Component)]
pub struct InterpolateRotation {
    pub src: Quat,
    pub dst: Quat,
    pub start: ControlFrame,
    pub end: ControlFrame,
}

impl InterpolateRotation {
    fn get(&self, now: ControlFrame) -> Quat {
        let d1 = self.end - self.start;
        let d2 = now - self.start;

        let s = d2.0 as f32 / d1.0 as f32;

        Quat::slerp(self.src, self.dst, f32::clamp(s, 0.0, 1.0))
    }
}

pub fn interpolate_translation(
    time: Res<Time>,
    conn: Res<ServerConnection>,
    mut commands: Commands,
    mut entities: Query<(Entity, &mut Transform, &InterpolateTranslation)>,
) {
    let now = time.last_update();

    for (entity, mut transform, interpolate) in &mut entities {
        let now = conn.control_fame() - (interpolate.end - interpolate.start);

        dbg!(time.last_update());
        dbg!(interpolate.end - interpolate.start);
        dbg!(now);

        transform.translation = interpolate.get(now);

        if now >= interpolate.end {
            commands.entity(entity).remove::<InterpolateTranslation>();
        }
    }
}

pub fn interpolate_rotation(
    time: Res<Time>,
    conn: Res<ServerConnection>,
    mut commands: Commands,
    mut entities: Query<(
        Entity,
        &mut Transform,
        Option<&mut ActorProperties>,
        &InterpolateRotation,
    )>,
) {
    for (entity, mut transform, props, interpolate) in &mut entities {
        let now = conn.control_fame() - (interpolate.end - interpolate.start);

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
    use game_common::world::control_frame::ControlFrame;
    use glam::Vec3;

    use super::InterpolateTranslation;

    #[test]
    fn interpolate_translation() {
        let lerp = InterpolateTranslation {
            src: Vec3::splat(0.0),
            dst: Vec3::splat(1.0),
            start: ControlFrame(0),
            end: ControlFrame(10),
        };

        assert_eq!(lerp.get(ControlFrame(0)), Vec3::splat(0.0));
        assert_eq!(lerp.get(ControlFrame(5)), Vec3::splat(0.5));
        assert_eq!(lerp.get(ControlFrame(10)), Vec3::splat(1.0));
    }
}
