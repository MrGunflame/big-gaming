use bevy_app::Plugin;

pub mod grid;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_startup_system(grid::spawn_grid);
        app.add_system(grid::synchronize_grid);
    }
}
