use bevy_ecs::prelude::Component;
use bevy_ecs::query::{With, Without};
use bevy_ecs::system::{Commands, Query, ResMut};
use game_asset::Assets;
use game_common::bundles::TransformBundle;
use game_common::components::transform::Transform;
use game_common::world::CELL_SIZE;
use game_render::camera::Camera;
use game_render::color::Color;
use game_render::mesh::Mesh;
use game_render::pbr::{PbrBundle, PbrMaterial};
use game_render::shape::{Face, Quad};
use game_render::texture::{Image, Images, TextureFormat};
use glam::{UVec2, Vec2, Vec3};
use image::{ImageBuffer, Rgba};

#[derive(Copy, Clone, Debug, Component)]
pub struct CellBorder {
    face: Face,
}

pub fn spawn_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PbrMaterial>>,
    mut images: ResMut<Images>,
) {
    // Assume the grid is uniform for now.
    assert_eq!(CELL_SIZE.x, CELL_SIZE.y);
    assert_eq!(CELL_SIZE.x, CELL_SIZE.z);

    let size = Vec2::new(CELL_SIZE.x, CELL_SIZE.x);

    let material = materials.insert(create_wall_material(&mut images));

    for face in [
        Face::Front,
        Face::Back,
        Face::Left,
        Face::Right,
        Face::Top,
        Face::Bottom,
    ] {
        let mesh = meshes.insert(Quad { size, face }.into());

        commands
            .spawn(PbrBundle {
                mesh,
                material: material.clone(),
                transform: TransformBundle::default(),
            })
            .insert(CellBorder { face });
    }
}

pub fn synchronize_grid(
    cameras: Query<&Transform, With<Camera>>,
    mut entities: Query<(&mut Transform, &CellBorder), Without<Camera>>,
) {
    let Ok(camera) = cameras.get_single() else {
        return;
    };

    for (mut transform, wall) in &mut entities {
        let mut offset = Vec3::ZERO;
        match wall.face {
            Face::Front => {
                offset.z = CELL_SIZE.z - (camera.translation.z % CELL_SIZE.z);
                offset.x = CELL_SIZE.x / 2.0 - (camera.translation.x % CELL_SIZE.x);
                offset.y = CELL_SIZE.y / 2.0 - (camera.translation.y % CELL_SIZE.y);
            }
            Face::Back => {
                offset.z = -(camera.translation.z % CELL_SIZE.z);
                offset.x = CELL_SIZE.x / 2.0 - (camera.translation.x % CELL_SIZE.x);
                offset.y = CELL_SIZE.y / 2.0 - (camera.translation.y % CELL_SIZE.y);
            }
            Face::Right => {
                offset.x = -(camera.translation.x % CELL_SIZE.x);
                offset.y = CELL_SIZE.y / 2.0 - (camera.translation.y % CELL_SIZE.y);
                offset.z = CELL_SIZE.z / 2.0 - (camera.translation.z % CELL_SIZE.z);
            }
            Face::Left => {
                offset.x = CELL_SIZE.x - (camera.translation.x % CELL_SIZE.x);
                offset.y = CELL_SIZE.y / 2.0 - (camera.translation.y % CELL_SIZE.y);
                offset.z = CELL_SIZE.z / 2.0 - (camera.translation.z % CELL_SIZE.z);
            }
            Face::Top => {
                offset.y = -(camera.translation.y % CELL_SIZE.y);
                offset.x = CELL_SIZE.x / 2.0 - (camera.translation.x % CELL_SIZE.x);
                offset.z = CELL_SIZE.z / 2.0 - (camera.translation.z % CELL_SIZE.z);
            }
            Face::Bottom => {
                offset.y = CELL_SIZE.y - (camera.translation.y % CELL_SIZE.y);
                offset.x = CELL_SIZE.x / 2.0 - (camera.translation.x % CELL_SIZE.x);
                offset.z = CELL_SIZE.z / 2.0 - (camera.translation.z % CELL_SIZE.z);
            }
        };

        transform.translation = camera.translation + offset;
    }
}

fn create_wall_material(images: &mut Images) -> PbrMaterial {
    let pixel = Rgba([255, 0, 0, 255]);
    let mut buf = ImageBuffer::from_pixel(32, 32, Rgba([0, 0, 255, 255]));
    for x in 0..buf.width() {
        buf.put_pixel(x, 0, pixel);
        buf.put_pixel(x, buf.height() - 1, pixel);
    }

    for y in 0..buf.height() {
        buf.put_pixel(0, y, pixel);
        buf.put_pixel(buf.width() - 1, y, pixel);
    }

    let image = Image::new(
        UVec2::new(buf.width(), buf.height()),
        TextureFormat::Rgba8UnormSrgb,
        buf.into_raw(),
    );

    let base_color_texture = images.insert(image);

    PbrMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(base_color_texture),
        ..Default::default()
    }
}
