use std::f32::consts::PI;

use bevy_ecs::system::Query;
use game_common::components::actor::{ActorModel, ActorProperties};
use game_common::components::animation::{AnimationQueue, Bone, Skeleton};
use game_common::components::transform::Transform;
use game_common::math::RotationExt;

use bevy_app::{App, Plugin};
use bevy_ecs::entity::Entity;
use bevy_ecs::query::Without;
use glam::{Quat, Vec3};

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(start_animations)
            .add_system(transform_bones)
            .add_system(sync_actor_rotations);
    }
}

fn start_animations(
    mut entities: Query<(Entity, &mut AnimationQueue, &ActorModel)>,
    mut segments: Query<&mut Transform, Without<ActorModel>>,
) {
    for (entity, mut queue, model) in &mut entities {
        while let Some(animation) = queue.pop() {
            tracing::trace!("entity {:?} started animation {:?}", entity, animation);

            for entity in model.entities.iter() {
                let mut transform = segments
                    .get_mut(*entity)
                    .expect("Entity defined in ActorModel is missing in segments");

                transform.rotation = Quat::from_axis_angle(Vec3::X, 90.0f32.to_radians());
            }
        }
    }
}

/// Update skeleton bones transform values based on their offsets.
fn transform_bones(mut skeletons: Query<&Skeleton>, mut bones: Query<(&Bone, &mut Transform)>) {
    let transform = Transform::default();

    for skeleton in &mut skeletons {
        update_bone(skeleton.root, transform, &mut bones);
    }
}

fn update_bone(
    root: Entity,
    mut root_transform: Transform,
    bones: &mut Query<(&Bone, &mut Transform)>,
) {
    let (bone, mut transform) = bones.get_mut(root).expect("invalid bone");
    transform.translation = root_transform.translation * bone.offset.translation;
    root_transform = *transform;

    // FIXME: Remove the clone and allocation.
    for bone in bone.children.clone().iter() {
        update_bone(*bone, root_transform, bones);
    }
}

fn sync_actor_rotations(mut actors: Query<(&mut Transform, &ActorProperties)>) {
    for (mut transform, props) in &mut actors {
        let mut pt = props.rotation.dir_vec();

        if pt.y == 1.0 {
            continue;
        }

        pt.y = 0.0;
        if !pt.is_normalized() {
            pt = pt.normalize();
        }

        let b = Vec3::Z;

        let mut angle = (pt.dot(b)).acos();
        if pt.x.is_sign_negative() {
            angle = -angle;
        }

        let rot = Quat::from_axis_angle(Vec3::Y, angle + PI);

        transform.rotation = rot;
    }
}
