use bevy::prelude::shape;
use bevy::prelude::Assets;
use bevy::prelude::Color;
use bevy::prelude::Commands;
use bevy::prelude::Component;
use bevy::prelude::Material;
use bevy::prelude::MaterialMeshBundle;
use bevy::prelude::MaterialPlugin;
use bevy::prelude::Mesh;
use bevy::prelude::Query;
use bevy::prelude::Res;
use bevy::prelude::ResMut;
use bevy::prelude::Resource;
use bevy::prelude::Transform;
use bevy::prelude::Vec3;
use bevy::prelude::With;
use bevy::prelude::Without;
use bevy::reflect::TypeUuid;
use bevy::render::render_resource::AsBindGroup;
use bevy::render::render_resource::ShaderRef;
use game_common::components::player::HostPlayer;

#[derive(Copy, Clone, Debug, Resource)]
pub struct Sky {
    pub distance: f32,
}

impl Sky {
    fn panes(&self) -> Vec<(Mesh, SkyPane)> {
        let distance = self.distance;

        // Top (Y plane)
        let top = shape::Box {
            min_x: -distance,
            max_x: distance,
            min_y: 0.0,
            max_y: 0.0,
            min_z: -distance,
            max_z: distance,
        };

        // Front (Z plane)
        let front = shape::Box {
            min_x: -distance,
            max_x: distance,
            min_y: -distance,
            max_y: distance,
            min_z: 0.0,
            max_z: 0.0,
        };

        // Right (X plane)
        let right = shape::Box {
            min_x: 0.0,
            max_x: 0.0,
            min_y: -distance,
            max_y: distance,
            min_z: -distance,
            max_z: distance,
        };

        vec![
            (top.into(), SkyPane::Y),
            (front.into(), SkyPane::Z),
            (right.into(), SkyPane::X),
            (front.into(), SkyPane::NegZ),
            (right.into(), SkyPane::NegX),
        ]
    }
}

#[derive(Clone, Debug, AsBindGroup, TypeUuid)]
#[uuid = "5a811417-336e-4e2b-b0a3-cf21d63e78f4"]
pub struct SkyMaterial {
    #[uniform(0)]
    depolarization_factor: f32,
    #[uniform(0)]
    mie_coefficient: f32,
    #[uniform(0)]
    mie_directional_g: f32,
    #[uniform(0)]
    mie_k_coefficient: Vec3,
    #[uniform(0)]
    mie_v: f32,
    #[uniform(0)]
    mie_zenith_length: f32,
    #[uniform(0)]
    num_molecules: f32,
    #[uniform(0)]
    primaries: Vec3,
    #[uniform(0)]
    rayleigh: f32,
    #[uniform(0)]
    rayleigh_zenith_length: f32,
    #[uniform(0)]
    refractive_index: f32,
    #[uniform(0)]
    sun_angular_diameter_degrees: f32,
    #[uniform(0)]
    sun_intensity_factor: f32,
    #[uniform(0)]
    sun_intensity_falloff_steepness: f32,
    #[uniform(0)]
    turbidity: f32,
}

impl Default for SkyMaterial {
    fn default() -> Self {
        Self {
            depolarization_factor: 0.035,
            mie_coefficient: 0.005,
            mie_directional_g: 0.8,
            mie_k_coefficient: Vec3::new(0.686, 0.678, 0.666),
            mie_v: 4.0,
            mie_zenith_length: 1.25e3,
            num_molecules: 2.542e25,
            primaries: Vec3::new(6.8e-7, 5.5e-7, 4.5e-7),
            rayleigh: 1.0,
            rayleigh_zenith_length: 8.4e3,
            refractive_index: 1.0003,
            sun_angular_diameter_degrees: 0.0093333,
            sun_intensity_factor: 1000.0,
            sun_intensity_falloff_steepness: 1.5,
            turbidity: 2.0,
        }
    }
}

impl Material for SkyMaterial {
    fn fragment_shader() -> ShaderRef {
        "sky.wgsl".into()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkyPlugin;

impl bevy::prelude::Plugin for SkyPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(Sky { distance: 1000.0 })
            .add_plugin(MaterialPlugin::<SkyMaterial>::default())
            .add_startup_system(setup_sky)
            .add_system(move_sky_pane);
    }
}

/// A pane of the sky.
#[derive(Copy, Clone, Debug, PartialEq, Component)]
enum SkyPane {
    /// The pane in the positive x (right) direction.
    X,
    NegX,
    Y,
    Z,
    NegZ,
}

fn setup_sky(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SkyMaterial>>,
    sky: Res<Sky>,
) {
    for (mesh, pane) in sky.panes() {
        commands
            .spawn(MaterialMeshBundle {
                mesh: meshes.add(mesh),
                material: materials.add(SkyMaterial::default()),
                ..Default::default()
            })
            .insert(pane);
    }
}

fn move_sky_pane(
    players: Query<&Transform, With<HostPlayer>>,
    mut panes: Query<(&mut Transform, &SkyPane), Without<HostPlayer>>,
    sky: Res<Sky>,
) {
    let Ok(player) = players.get_single() else {
        return;
    };

    let distance = sky.distance;

    for (mut transform, pane) in &mut panes {
        match pane {
            SkyPane::X => transform.translation = player.translation + Vec3::X * distance,
            SkyPane::NegX => transform.translation = player.translation - Vec3::X * distance,
            SkyPane::Y => transform.translation = player.translation + Vec3::Y * distance,
            SkyPane::Z => transform.translation = player.translation + Vec3::Z * distance,
            SkyPane::NegZ => transform.translation = player.translation - Vec3::Z * distance,
        }
    }
}
