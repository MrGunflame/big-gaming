use bevy_ecs::prelude::{Bundle, Component, Entity};
use bevy_ecs::system::{Commands, Query, Res, ResMut};
use game_common::bundles::TransformBundle;
use game_common::components::actor::{ActorProperties, MovementSpeed};
use game_common::components::combat::Health;
use game_common::components::inventory::Inventory;
use game_common::components::player::HostPlayer;
use game_common::components::race::RaceId;
use game_common::components::transform::Transform;
use game_core::modules::Modules;
use game_input::hotkeys::Hotkeys;
use game_scene::{SceneBundle, Scenes};
use glam::Vec3;

use crate::net::interpolate::{InterpolateRotation, InterpolateTranslation};
use crate::plugins::actions::ActiveActions;
use crate::utils::extract_actor_rotation;

#[derive(Clone, Debug, Component)]
pub struct LoadActor {
    pub transform: Transform,
    pub race: RaceId,
    pub health: Health,
    pub host: bool,
    pub inventory: Inventory,
}

#[derive(Bundle)]
struct ActorBundle {
    #[bundle]
    scene: SceneBundle,
    speed: MovementSpeed,
    props: ActorProperties,
    inventory: Inventory,

    interpolate_translation: InterpolateTranslation,
    interpolate_rotation: InterpolateRotation,
}

pub fn load_actor(
    mut commands: Commands,
    entities: Query<(Entity, &LoadActor)>,
    mut scenes: ResMut<Scenes>,
    mut active_actions: ResMut<ActiveActions>,
    mut hotkeys: ResMut<Hotkeys>,
    modules: Res<Modules>,
) {
    for (entity, actor) in &entities {
        tracing::trace!("spawning actor at {:?}", actor.transform.translation);

        dbg!(actor);

        let mut cmds = commands.entity(entity);
        cmds.remove::<LoadActor>();

        cmds.insert(ActorBundle {
            scene: SceneBundle {
                scene: scenes.load("../assets/bricks.glb"),
                transform: TransformBundle {
                    transform: Transform {
                        translation: actor.transform.translation,
                        rotation: extract_actor_rotation(actor.transform.rotation),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            },
            inventory: actor.inventory.clone(),
            speed: MovementSpeed::default(),
            props: ActorProperties {
                rotation: actor.transform.rotation,
                eyes: Vec3::splat(0.0),
            },
            interpolate_translation: InterpolateTranslation::default(),
            interpolate_rotation: InterpolateRotation::default(),
        });

        if actor.host {
            cmds.insert(HostPlayer);
        }

        // Load actions
        for item in &actor.inventory {
            let module = modules.get(item.item.id.0.module).unwrap();
            let record = module.records.get(item.item.id.0.record).unwrap();
            let item = record.body.as_item().unwrap();

            for action in &item.actions {
                let module = modules.get(action.module).unwrap();
                let record = module.records.get(action.record).unwrap().clone();

                active_actions.register(&mut hotkeys, action.module, record);
            }
        }
    }
}
