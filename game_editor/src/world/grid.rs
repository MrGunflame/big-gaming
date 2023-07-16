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
            .insert(CellBorder {
                face: face.inverse(),
            });
    }
}

pub fn synchronize_grid(
    cameras: Query<&Transform, With<Camera>>,
    mut entities: Query<(&mut Transform, &CellBorder), Without<Camera>>,
) {
    let Ok(camera) = cameras.get_single() else {
        return;
    };

    for (mut transform, border) in &mut entities {
        transform.translation = calculate_translation(camera.translation, border.face);
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

fn calculate_translation(origin: Vec3, face: Face) -> Vec3 {
    let mut offset = Vec3::ZERO;
    match face {
        Face::Front => {
            offset.z = CELL_SIZE.z - (origin.z % CELL_SIZE.z);
            offset.x = CELL_SIZE.x / 2.0 - (origin.x % CELL_SIZE.x);
            offset.y = CELL_SIZE.y / 2.0 - (origin.y % CELL_SIZE.y);
        }
        Face::Back => {
            offset.z = -(origin.z % CELL_SIZE.z);
            offset.x = CELL_SIZE.x / 2.0 - (origin.x % CELL_SIZE.x);
            offset.y = CELL_SIZE.y / 2.0 - (origin.y % CELL_SIZE.y);
        }
        Face::Right => {
            offset.x = CELL_SIZE.x - (origin.x % CELL_SIZE.x);
            offset.y = CELL_SIZE.y / 2.0 - (origin.y % CELL_SIZE.y);
            offset.z = CELL_SIZE.z / 2.0 - (origin.z % CELL_SIZE.z);
        }
        Face::Left => {
            offset.x = -(origin.x % CELL_SIZE.x);
            offset.y = CELL_SIZE.y / 2.0 - (origin.y % CELL_SIZE.y);
            offset.z = CELL_SIZE.z / 2.0 - (origin.z % CELL_SIZE.z);
        }
        Face::Top => {
            offset.y = CELL_SIZE.y - (origin.y % CELL_SIZE.y);
            offset.x = CELL_SIZE.x / 2.0 - (origin.x % CELL_SIZE.x);
            offset.z = CELL_SIZE.z / 2.0 - (origin.z % CELL_SIZE.z);
        }
        Face::Bottom => {
            offset.y = -(origin.y % CELL_SIZE.y);
            offset.x = CELL_SIZE.x / 2.0 - (origin.x % CELL_SIZE.x);
            offset.z = CELL_SIZE.z / 2.0 - (origin.z % CELL_SIZE.z);
        }
    };

    origin + offset
}

#[cfg(test)]
mod tests {
    use game_common::world::CELL_SIZE;
    use game_render::shape::Face;
    use glam::Vec3;

    use crate::world::grid::calculate_translation;

    #[test]
    fn calculate_translation_front() {
        let origin = Vec3::new(1.0, 2.0, 3.0);
        let face = Face::Front;

        let offset = calculate_translation(origin, face);
        assert_eq!(
            offset,
            Vec3::new(CELL_SIZE.x / 2.0, CELL_SIZE.y / 2.0, CELL_SIZE.z)
        );
    }

    #[test]
    fn calculate_translation_back() {
        let origin = Vec3::new(1.0, 2.0, 3.0);
        let face = Face::Back;

        let offset = calculate_translation(origin, face);
        assert_eq!(offset, Vec3::new(CELL_SIZE.x / 2.0, CELL_SIZE.y / 2.0, 0.0));
    }

    #[test]
    fn calculate_translation_left() {
        let origin = Vec3::new(1.0, 2.0, 3.0);
        let face = Face::Left;

        let offset = calculate_translation(origin, face);
        assert_eq!(offset, Vec3::new(0.0, CELL_SIZE.y / 2.0, CELL_SIZE.z / 2.0));
    }

    #[test]
    fn calculate_translation_right() {
        let origin = Vec3::new(1.0, 2.0, 3.0);
        let face = Face::Right;

        let offset = calculate_translation(origin, face);
        assert_eq!(
            offset,
            Vec3::new(CELL_SIZE.x, CELL_SIZE.y / 2.0, CELL_SIZE.z / 2.0)
        );
    }

    #[test]
    fn calculate_translation_top() {
        let origin = Vec3::new(1.0, 2.0, 3.0);
        let face = Face::Top;

        let offset = calculate_translation(origin, face);
        assert_eq!(
            offset,
            Vec3::new(CELL_SIZE.x / 2.0, CELL_SIZE.y, CELL_SIZE.z / 2.0)
        );
    }

    #[test]
    fn calculate_translation_bottom() {
        let origin = Vec3::new(1.0, 2.0, 3.0);
        let face = Face::Bottom;

        let offset = calculate_translation(origin, face);
        assert_eq!(offset, Vec3::new(CELL_SIZE.x / 2.0, 0.0, CELL_SIZE.z / 2.0));
    }
}
