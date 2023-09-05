use crate::world::terrain::TerrainMesh;
use crate::world::CellId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Terrain;

#[derive(Clone, Debug)]
pub struct LoadTerrain {
    pub cell: CellId,
    pub mesh: TerrainMesh,
}
