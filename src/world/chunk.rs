use bevy::prelude::{Commands, Mesh, PbrBundle};
use bevy_rapier3d::parry::shape;

use super::ChunkId;

pub struct Chunk {
    id: ChunkId,
}
