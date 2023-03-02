use bevy::prelude::{
    shape, Assets, Color, Commands, Component, Entity, Mesh, PbrBundle, Quat, Query, ResMut,
    Resource, StandardMaterial, Transform, Vec3, With,
};
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;

use super::{EntityOptions, COLOR_X, COLOR_Y, COLOR_Z};

pub struct Axis {
    pub direction: Vec3,
    pub length: f32,
}

impl Axis {
    fn mesh(&self) -> Mesh {
        #[cfg(debug_assertions)]
        assert!(self.direction.is_normalized());

        // let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        // mesh.insert_attribute(
        //     Mesh::ATTRIBUTE_POSITION,
        //     vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]],
        // );
        // mesh.set_indices(Some(Indices::U32(vec![0, 1, 2])));

        // mesh.insert_attribute(
        //     Mesh::ATTRIBUTE_NORMAL,
        //     vec![[0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
        // );
        // // mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 1.0], ]);

        // let mut positions: Vec<[f32; 3]> = Vec::new();

        // Cone
        // let origin = Vec3::new(0.0, 0.0, 0.0);
        // let radius = 5.0;
        // let height = 5.0;
        // let subdivisions = 3;

        // let tip = origin + Vec3::new(0.0, height, 0.0);
        // positions.push([tip.x, tip.y, tip.z]);

        // for side in 0..=subdivisions {
        //     positions.push([]);
        // }

        let mesh = shape::Box {
            min_x: -0.01,
            max_x: (self.direction.x * self.length).max(0.01),
            min_y: -0.01,
            max_y: (self.direction.y * self.length).max(0.01),
            min_z: -0.01,
            max_z: (self.direction.z * self.length).max(0.01),
        };

        mesh.into()
    }
}

impl From<Axis> for Mesh {
    fn from(value: Axis) -> Self {
        value.mesh()
    }
}

pub fn render_axes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    axes: Query<Entity, With<AxisMarker>>,
    entities: Query<(&Transform, &EntityOptions)>,
) {
    for entity in &axes {
        commands.entity(entity).despawn();
    }

    for (transform, options) in &entities {
        // Show only on selected entities.
        if !options.selected {
            continue;
        }

        for (direction, color) in [(Vec3::X, COLOR_X), (Vec3::Y, COLOR_Y), (Vec3::Z, COLOR_Z)] {
            commands
                .spawn(PbrBundle {
                    mesh: meshes.add(
                        Axis {
                            direction,
                            length: 1.0,
                        }
                        .into(),
                    ),
                    material: materials.add(color.into()),
                    transform: Transform {
                        translation: transform.translation,
                        rotation: Quat::IDENTITY,
                        scale: Vec3::splat(1.0),
                    },
                    ..Default::default()
                })
                .insert(AxisMarker);
        }
    }
}

#[derive(Copy, Clone, Debug, Component)]
pub struct AxisMarker;
