use bevy_ecs::component::Component;

use crate::world::terrain::TerrainMesh;
use crate::world::CellId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct Terrain;

#[derive(Clone, Debug, Component)]
pub struct LoadTerrain {
    pub cell: CellId,
    pub mesh: TerrainMesh,
}
