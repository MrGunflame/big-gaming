//! Chunk related systems

use bevy::prelude::{Plugin, ResMut, Stage, Transform, With};
use bevy_ecs::entity::Entity;
use bevy_ecs::system::Query;
use common::components::player::Player;
use common::components::transform::PreviousTransform;
use common::world::chunk::{ChunkId, ChunkRegistry};

#[derive(Clone, Debug)]
pub struct ChunkPlugin {
    registry: ChunkRegistry,
}

impl ChunkPlugin {
    #[inline]
    pub fn new(registry: ChunkRegistry) -> Self {
        Self { registry }
    }
}

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(self.registry.clone())
            .add_system(transfer_entities)
            .add_system(update_player_chunks);
    }
}

/// Transfer entities between chunks.
fn transfer_entities(
    mut chunk_registry: ResMut<ChunkRegistry>,
    mut entities: Query<(Entity, &Transform, &PreviousTransform)>,
) {
    for (entity, transform, previous_transform) in &mut entities {
        let id = ChunkId::from(transform.translation);
        let prev_id = ChunkId::from(previous_transform.translation);

        // Chunk did not change.
        if id == prev_id {
            continue;
        }

        chunk_registry.get(id).insert(entity);
        chunk_registry.get(prev_id).remove(entity);
    }
}

fn update_player_chunks(
    mut chunk_registry: ResMut<ChunkRegistry>,
    mut players: Query<&Transform, With<Player>>,
) {
    for player in &mut players {
        let id = ChunkId::from(player.translation);

        chunk_registry.load(id);
    }
}

struct ChunkStage;

impl Stage for ChunkStage {
    fn run(&mut self, world: &mut bevy::prelude::World) {}
}
