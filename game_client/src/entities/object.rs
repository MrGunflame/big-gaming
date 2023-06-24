use bevy_ecs::prelude::{Component, Entity};
use bevy_ecs::system::{Commands, Query, Res, ResMut};
use game_common::bundles::TransformBundle;
use game_common::components::object::ObjectId;
use game_common::components::transform::Transform;
use game_core::modules::Modules;
use game_scene::{SceneBundle, Scenes};

#[derive(Clone, Debug, Component)]
pub struct LoadObject {
    pub id: ObjectId,
    pub transform: Transform,
}

pub fn load_object(
    mut commands: Commands,
    entities: Query<(Entity, &LoadObject)>,
    modules: Res<Modules>,
    mut scenes: ResMut<Scenes>,
) {
    for (entity, object) in &entities {
        tracing::trace!("spawning object at {:?}", object.transform.translation);

        let module = modules.get(object.id.0.module).unwrap();
        let record = module.records.get(object.id.0.record).unwrap();
        let obj = record.body.as_object().unwrap();

        let mut cmds = commands.entity(entity);
        cmds.remove::<LoadObject>();

        cmds.insert(SceneBundle {
            scene: scenes.load(obj.uri.as_ref()),
            transform: TransformBundle {
                transform: object.transform,
                ..Default::default()
            },
        });
    }
}
