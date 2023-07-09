use bevy::prelude::{
    shape, Assets, Commands, Component, Entity, Mesh, PbrBundle, Quat, Query, ResMut,
    StandardMaterial, Transform, Vec3, With,
};

use super::{EntityOptions, COLOR_X, COLOR_Y, COLOR_Z};

pub struct Axis {
    pub direction: Vec3,
    pub length: f32,
}

impl Axis {
    fn mesh(&self) -> Mesh {
        #[cfg(debug_assertions)]
        assert!(self.direction.is_normalized());

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
