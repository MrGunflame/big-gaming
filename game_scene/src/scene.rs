use game_common::components::transform::Transform;
use game_core::hierarchy::{Entity, TransformHierarchy};
use game_render::color::Color;
use game_render::entities::Object;
use game_render::light::{DirectionalLight, PointLight, SpotLight};
use game_render::mesh::Mesh;
use game_render::pbr::{AlphaMode, PbrMaterial};
use game_render::texture::Image;
use game_render::{shape, Renderer};
use game_tracing::trace_span;

use super::Entities;

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub nodes: Vec<Node>,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub images: Vec<Image>,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub transform: Transform,
    pub body: NodeBody,
}

#[derive(Copy, Clone, Debug)]
pub enum NodeBody {
    Object(ObjectNode),
    DirectionalLight(DirectionalLightNode),
    PointLight(PointLightNode),
    SpotLight(SpotLightNode),
}

#[derive(Copy, Clone, Debug)]
pub struct ObjectNode {
    pub mesh: usize,
    pub material: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct DirectionalLightNode {
    pub color: Color,
    pub illuminance: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct PointLightNode {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct SpotLightNode {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    pub inner_cutoff: f32,
    pub outer_cutoff: f32,
}

impl Scene {
    pub(crate) fn spawn(
        self,
        renderer: &mut Renderer,
        parent: Entity,
        hierarchy: &mut TransformHierarchy,
        entities: &mut Entities,
    ) -> Vec<Entity> {
        let _span = trace_span!("Scene::spawn").entered();

        let mut meshes = Vec::new();
        for mesh in self.meshes {
            let id = renderer.meshes.insert(mesh);
            meshes.push(id);
        }

        let mut images = Vec::new();
        for image in self.images {
            let id = renderer.images.insert(image);
            images.push(id);
        }

        let mut materials = Vec::new();
        for material in self.materials {
            let id = renderer.materials.insert(PbrMaterial {
                alpha_mode: material.alpha_mode,
                base_color: material.base_color,
                base_color_texture: material.base_color_texture.map(|index| images[index]),
                normal_texture: material.normal_texture.map(|index| images[index]),
                roughness: material.roughness,
                metallic: material.metallic,
                metallic_roughness_texture: material
                    .metallic_roughness_texture
                    .map(|index| images[index]),
                reflectance: material.reflectance,
            });
            materials.push(id);
        }

        let mut children = Vec::new();

        for node in &self.nodes {
            let key = hierarchy.append(Some(parent), node.transform);

            match node.body {
                NodeBody::Object(object) => {
                    let id = renderer.entities.objects.insert(Object {
                        transform: node.transform,
                        mesh: meshes[object.mesh],
                        material: materials[object.material],
                    });

                    entities.objects.insert(key, id);
                }
                NodeBody::DirectionalLight(light) => {
                    let id = renderer
                        .entities
                        .directional_lights
                        .insert(DirectionalLight {
                            transform: node.transform,
                            color: light.color,
                            illuminance: light.illuminance,
                        });

                    entities.directional_lights.insert(key, id);
                }
                NodeBody::PointLight(light) => {
                    let id = renderer.entities.point_lights.insert(PointLight {
                        transform: node.transform,
                        color: light.color,
                        intensity: light.intensity,
                        radius: light.radius,
                    });

                    entities.point_lights.insert(key, id);
                }
                NodeBody::SpotLight(light) => {
                    let id = renderer.entities.spot_lights.insert(SpotLight {
                        transform: node.transform,
                        color: light.color,
                        intensity: light.intensity,
                        radius: light.radius,
                        inner_cutoff: light.inner_cutoff,
                        outer_cutoff: light.outer_cutoff,
                    });

                    entities.spot_lights.insert(key, id);
                }
            }

            children.push(key);
        }

        // Local Coordinate axes for debugging
        for (mesh, color) in [
            (
                shape::Box {
                    min_x: 0.0,
                    max_x: 2.0,
                    min_y: -0.1,
                    max_y: 0.1,
                    min_z: -0.1,
                    max_z: 0.1,
                },
                Color::RED,
            ),
            (
                shape::Box {
                    min_x: -0.1,
                    max_x: 0.1,
                    min_y: 0.0,
                    max_y: 2.0,
                    min_z: -0.1,
                    max_z: 0.1,
                },
                Color::GREEN,
            ),
            (
                shape::Box {
                    min_x: -0.1,
                    max_x: 0.1,
                    min_y: -0.1,
                    max_y: 0.1,
                    min_z: 0.0,
                    max_z: 2.0,
                },
                Color::BLUE,
            ),
        ] {
            renderer.entities.objects.insert(Object {
                transform: Default::default(),
                mesh: renderer.meshes.insert(mesh.into()),
                material: renderer.materials.insert(PbrMaterial {
                    base_color: color,
                    ..Default::default()
                }),
            });
        }

        children
    }
}

// The same as `game_render::PbrMaterial`, but with different image handles.
#[derive(Copy, Clone, Debug)]
pub struct Material {
    pub alpha_mode: AlphaMode,
    pub base_color: Color,
    pub base_color_texture: Option<usize>,
    pub normal_texture: Option<usize>,
    pub roughness: f32,
    pub metallic: f32,
    pub metallic_roughness_texture: Option<usize>,
    pub reflectance: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            alpha_mode: AlphaMode::default(),
            base_color: Color::WHITE,
            base_color_texture: None,
            normal_texture: None,
            roughness: 0.5,
            metallic: 0.0,
            metallic_roughness_texture: None,
            reflectance: 0.5,
        }
    }
}
