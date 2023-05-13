use bevy_app::App;
use bevy_ecs::system::Commands;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::material::MaterialMeshBundle;
use game_render::{shape, RenderPlugin};
use game_window::Window;

fn main() {
    let mut app = App::new();
    app.add_plugin(RenderPlugin);
    app.add_startup_system(setup);

    app.run();
}

fn setup(mut cmds: Commands) {
    let id = cmds
        .spawn(Window {
            title: "test".to_owned(),
        })
        .id();

    cmds.spawn(Camera {
        target: RenderTarget::Window(id),
        projection: Projection::default(),
    });

    cmds.spawn(MaterialMeshBundle {
        mesh: shape::Box {
            min_x: -0.5,
            max_x: 0.5,
            min_y: -0.5,
            max_y: 0.5,
            min_z: -0.5,
            max_z: 0.5,
        }
        .into(),
    });
}
