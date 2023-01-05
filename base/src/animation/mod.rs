use bevy::prelude::{Entity, Plugin, Quat, Query, Transform, Vec3, Without};
use common::components::actor::ActorModel;
use common::components::animation::AnimationQueue;

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(start_animations);
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
