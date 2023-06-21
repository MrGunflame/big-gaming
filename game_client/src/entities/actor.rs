use bevy_ecs::prelude::{Component, Entity};
use bevy_ecs::system::{Commands, Query, ResMut};
use game_common::bundles::TransformBundle;
use game_common::components::actor::{ActorProperties, MovementSpeed};
use game_common::components::combat::Health;
use game_common::components::player::HostPlayer;
use game_common::components::race::RaceId;
use game_common::components::transform::Transform;
use game_scene::{SceneBundle, Scenes};
use glam::Vec3;

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

        let mut cmds = commands.spawn(SceneBundle {
            scene: scenes.load("../assets/metal.glb"),
            transform: TransformBundle {
                transform: actor.transform,
                ..Default::default()
            },
        });

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
