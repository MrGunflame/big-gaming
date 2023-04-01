#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TerrainPlugin;

impl bevy::prelude::Plugin for TerrainPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {}
}
