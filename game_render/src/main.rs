use bevy_app::App;
use bevy_ecs::system::Commands;
use game_render::camera::{Camera, Projection, RenderTarget, Transform};
use game_render::material::{Material, MaterialMeshBundle};
use game_render::{shape, RenderPlugin};
use game_window::Window;
use glam::Vec3;

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
    })
    .insert(Transform::default());

    let img = image::io::Reader::open("../assets/Baker.png")
        .unwrap()
        .decode()
        .unwrap()
        .to_rgba8();

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
        material: Material {
            color: [1.0, 0.0, 0.0, 1.0],
            color_texture: img.clone(),
        },
        computed_material: Default::default(),
        computed_mesh: Default::default(),
    })
    .insert(Transform {
        translation: Vec3::new(0.0, 1.0, -5.0),
        ..Default::default()
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
        material: Material {
            color: [1.0, 1.0, 1.0, 1.0],
            color_texture: img,
        },
        computed_material: Default::default(),
        computed_mesh: Default::default(),
    })
    .insert(Transform {
        translation: Vec3::new(1.0, -0.5, -4.0),
        ..Default::default()
    });

    // cmds.spawn(MaterialMeshBundle {
    //     mesh: shape::Plane { size: 100.0 }.into(),
    // })
    // .insert(Transform {
    //     translation: Vec3::new(0.0, -5.0, 0.0),
    //     ..Default::default()
    // });
}
