use bevy_ecs::prelude::{Component, Entity};
use bevy_ecs::system::{Commands, Query, Res};
use game_common::components::actor::ActorProperties;
use game_common::components::transform::Transform;
use game_common::world::control_frame::ControlFrame;
use game_core::time::Time;
use glam::{Quat, Vec3};

use crate::utils::extract_actor_rotation;

use super::ServerConnection;

// No `Copy` impl to prevent accidental move-out.
#[derive(Clone, Debug, Default, Component)]
pub struct InterpolateTranslation {
    // Note that the interpolation system runs directly after apply the world
    // delta. We can't afford to wait for `Commands` to insert the component,
    // so we use a `None` value as a no-interplation value.
    inner: Option<InterpolateTranslationInner>,
}

impl InterpolateTranslation {
    pub fn set(&mut self, src: Vec3, dst: Vec3, start: ControlFrame, end: ControlFrame) {
        self.inner = Some(InterpolateTranslationInner {
            src,
            dst,
            start,
            end,
        });
    }
}

#[derive(Copy, Clone, Debug)]
struct InterpolateTranslationInner {
    src: Vec3,
    dst: Vec3,
    start: ControlFrame,
    end: ControlFrame,
}

impl InterpolateTranslationInner {
    fn get(&self, now: ControlFrame) -> Vec3 {
        let d1 = self.end - self.start;
        let d2 = now - self.start;

        let s = d2.0 as f32 / d1.0 as f32;

        Vec3::lerp(self.src, self.dst, f32::clamp(s, 0.0, 1.0))
    }
}

// No `Copy` impl to prevent accidental move-out.
#[derive(Clone, Debug, Default, Component)]
pub struct InterpolateRotation {
    // Note that the interpolation system runs directly after apply the world
    // delta. We can't afford to wait for `Commands` to insert the component,
    // so we use a `None` value as a no-interplation value.
    inner: Option<InterpolateRotationInner>,
}

impl InterpolateRotation {
    pub fn set(&mut self, src: Quat, dst: Quat, start: ControlFrame, end: ControlFrame) {
        self.inner = Some(InterpolateRotationInner {
            src,
            dst,
            start,
            end,
        });
    }
}

#[derive(Copy, Clone, Debug)]
struct InterpolateRotationInner {
    src: Quat,
    dst: Quat,
    start: ControlFrame,
    end: ControlFrame,
}

impl InterpolateRotationInner {
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
    mut entities: Query<(Entity, &mut Transform, &mut InterpolateTranslation)>,
) {
    let now = time.last_update();

    for (entity, mut transform, mut interpolate) in &mut entities {
        let Some(inner) = interpolate.inner else {
            continue;
        };

        let now = conn.control_fame() - (inner.end - inner.start);

        transform.translation = inner.get(now);

        if now >= inner.end {
            interpolate.inner = None;
        }
    }
}

pub fn interpolate_rotation(
    time: Res<Time>,
    conn: Res<ServerConnection>,
    mut entities: Query<(
        Entity,
        &mut Transform,
        Option<&mut ActorProperties>,
        &mut InterpolateRotation,
    )>,
) {
    for (entity, mut transform, props, mut interpolate) in &mut entities {
        let Some(inner) = interpolate.inner else {
            continue;
        };

        let now = conn.control_fame() - (inner.end - inner.start);

        if let Some(mut props) = props {
            // props.rotation = interpolate.get(now);
            // transform.rotation = extract_actor_rotation(props.rotation);
            props.rotation = inner.dst;
            transform.rotation = inner.dst;
        } else {
            // transform.rotation = interpolate.get(now);

            transform.rotation = inner.dst;
        }

        if now >= inner.end {
            interpolate.inner = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use game_common::world::control_frame::ControlFrame;
    use glam::Vec3;

    use crate::net::interpolate::InterpolateTranslationInner;

    #[test]
    fn interpolate_translation() {
        let lerp = InterpolateTranslationInner {
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
