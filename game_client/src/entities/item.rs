use bevy_ecs::prelude::{Bundle, Component, Entity};
use bevy_ecs::system::{Commands, Query, Res, ResMut};
use game_common::bundles::TransformBundle;
use game_common::components::items::ItemId;
use game_common::components::transform::Transform;
use game_core::modules::Modules;
use game_scene::{SceneBundle, Scenes};

use crate::net::interpolate::{InterpolateRotation, InterpolateTranslation};

#[derive(Clone, Debug, Component)]
pub struct LoadItem {
    pub id: ItemId,
    pub transform: Transform,
}

#[derive(Bundle)]
struct ItemBundle {
    #[bundle]
    scene: SceneBundle,

    interpolate_translation: InterpolateTranslation,
    interpolate_rotation: InterpolateRotation,
}

pub fn load_item(
    mut commands: Commands,
    entities: Query<(Entity, &LoadItem)>,
    mut scenes: ResMut<Scenes>,
    modules: Res<Modules>,
) {
    for (entity, item) in &entities {
        tracing::trace!("spawning item at {:?}", item.transform.translation);
        let mut cmds = commands.entity(entity);
        cmds.remove::<LoadItem>();

        cmds.insert(ItemBundle {
            scene: SceneBundle {
                scene: scenes.load("../assets/bricks.glb"),
                transform: TransformBundle {
                    transform: item.transform,
                    ..Default::default()
                },
            },
            interpolate_translation: InterpolateTranslation::default(),
            interpolate_rotation: InterpolateRotation::default(),
        });
    }
}
