use bevy_ecs::prelude::{Component, Entity};
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
use glam::{Quat, Vec3};

use crate::plugins::actions::ActiveActions;

#[derive(Clone, Debug, Component)]
pub struct LoadActor {
    pub transform: Transform,
    pub race: RaceId,
    pub health: Health,
    pub host: bool,
    pub inventory: Inventory,
}

pub fn load_actor(
    mut commands: Commands,
    entities: Query<(Entity, &LoadActor)>,
    mut scenes: ResMut<Scenes>,
    mut active_actions: ResMut<ActiveActions>,
    mut hotkeys: ResMut<Hotkeys>,
    mut modules: Res<Modules>,
) {
    for (entity, actor) in &entities {
        tracing::trace!("spawning actor at {:?}", actor.transform.translation);

        // Extract the rotation angle around Y, removing all other
        // components.
        let mut direction = actor.transform.rotation * -Vec3::Z;
        // Clamp in range of [-1, -1] in case direction is slightly above due
        // to FP error creep.
        direction.y = direction.y.clamp(-1.0, 1.0);
        let angle = if direction.x.is_sign_negative() {
            -direction.y.asin()
        } else {
            direction.y.asin()
        };

        let mut cmds = commands.entity(entity);
        cmds.remove::<LoadActor>();

        cmds.insert(SceneBundle {
            scene: scenes.load("../assets/bricks.glb"),
            transform: TransformBundle {
                transform: Transform {
                    translation: actor.transform.translation,
                    rotation: Quat::from_axis_angle(Vec3::Y, angle),
                    ..Default::default()
                },
                ..Default::default()
            },
        });

        cmds.insert(actor.inventory.clone());
        cmds.insert(MovementSpeed::default());
        cmds.insert(ActorProperties {
            rotation: actor.transform.rotation,
            eyes: Vec3::splat(0.0),
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
