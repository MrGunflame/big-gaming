use bevy_ecs::prelude::{Component, Entity};
use bevy_ecs::system::{Commands, Query, ResMut};
use game_common::bundles::TransformBundle;
use game_common::components::actor::{ActorProperties, MovementSpeed};
use game_common::components::combat::Health;
use game_common::components::player::HostPlayer;
use game_common::components::race::RaceId;
use game_common::components::transform::Transform;
use game_scene::{SceneBundle, Scenes};
use glam::{Quat, Vec3};

#[derive(Clone, Debug, Component)]
pub struct LoadActor {
    pub transform: Transform,
    pub race: RaceId,
    pub health: Health,
    pub host: bool,
}

pub fn load_actor(
    mut commands: Commands,
    entities: Query<(Entity, &LoadActor)>,
    mut scenes: ResMut<Scenes>,
) {
    for (entity, actor) in &entities {
        tracing::trace!("spawning actor at {:?}", actor.transform.translation);

        dbg!(actor.transform);
        dbg!(actor.transform.rotation * -Vec3::Z);

        let direction = actor.transform.rotation * -Vec3::Z;
        let angle = if direction.x.is_sign_negative() {
            -direction.y.asin()
        } else {
            direction.y.asin()
        };

        dbg!(angle);
        dbg!(Quat::from_axis_angle(Vec3::Y, angle));
        dbg!(Quat::from_axis_angle(Vec3::Y, angle) * -Vec3::Z);

        let mut cmds = commands.spawn(SceneBundle {
            scene: scenes.load("../assets/metal.glb"),
            transform: TransformBundle {
                transform: Transform {
                    translation: actor.transform.translation,
                    rotation: Quat::from_axis_angle(Vec3::Y, angle),
                    ..Default::default()
                },
                ..Default::default()
            },
        });

        dbg!(actor.transform.rotation.to_axis_angle());

        cmds.insert(MovementSpeed::default());
        cmds.insert(ActorProperties {
            rotation: actor.transform.rotation,
            eyes: Vec3::splat(0.0),
        });

        if actor.host {
            cmds.insert(HostPlayer);
        }

        commands.entity(entity).remove::<LoadActor>();
    }
}
