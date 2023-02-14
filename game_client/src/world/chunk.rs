use bevy::prelude::{Entity, Plugin, Query, ResMut, Transform, With};
use game_common::world::chunk::{ChunkId, ChunkRegistry};

// pub struct ChunkPlugin;

// impl Plugin for ChunkPlugin {
//     fn build(&self, app: &mut bevy::prelude::App) {
//         app.insert_resource(ChunkRegistry::new())
//             .add_system(transfer_entity)
//             .add_system(update_player_chunks);
//     }
// }

// /// Transfer an entity between chunks.
// fn transfer_entity(
//     mut chunks: ResMut<ChunkRegistry>,
//     mut entities: Query<(Entity, &Transform, &PreviousTransform)>,
// ) {
//     for (entity, transform, prev_transform) in &mut entities {
//         let id = ChunkId::from(transform.translation);
//         let prev_id = ChunkId::from(prev_transform.0.translation);

//         // Chunk did not change.
//         if id == prev_id {
//             continue;
//         }

//         // Transfer the entity.
//         chunks.get(id).insert(entity);
//         chunks.get(prev_id).remove(entity);
//     }
// }

// fn update_player_chunks(
//     chunks: ResMut<ChunkRegistry>,
//     mut players: Query<&Transform, With<PlayerCharacter>>,
// ) {
//     for player in &mut players {
//         let id = ChunkId::from(player.translation);

//         chunks.load(id);
//     }
// }
