use bevy::prelude::{Entity, Plugin, Quat, Query, Transform, Vec3, Without};
use game_common::components::actor::ActorModel;
use game_common::components::animation::{AnimationQueue, Bone, Skeleton};

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(start_animations).add_system(transform_bones);
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
    *transform = root_transform * bone.offset;
    root_transform = *transform;

    // FIXME: Remove the clone and allocation.
    for bone in bone.children.clone().iter() {
        update_bone(*bone, root_transform, bones);
    }
}
