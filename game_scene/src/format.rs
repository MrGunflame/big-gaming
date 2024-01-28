// TODO: We might want to eventaully merge this with our custom
// model format.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use game_common::components::{Color, Transform};
use game_gltf::types::{GltfMaterial, GltfMesh, TextureIndex};
use game_render::mesh::{Indices, Mesh};
use game_render::pbr::{AlphaMode, PbrMaterial};
use game_render::texture::ImageId;
use game_render::Renderer;
use glam::{Quat, Vec2, Vec3};
use serde::Deserialize;

use crate::scene2::{Key, SceneGraph};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRoot {
    pub nodes: Vec<Node>,
}

impl SceneRoot {
    pub fn spawn(self, renderer: &mut Option<&mut Renderer>, parent: Key, graph: &mut SceneGraph) {
        let mut parents = HashMap::new();

        for (index, node) in self.nodes.iter().enumerate() {
            if let Some(parent) = node.parent {
                parents.insert(index, parent);
            }

            let transform = Transform {
                translation: Vec3::from_array(node.translation),
                rotation: Quat::from_array(node.rotation),
                scale: Vec3::from_array(node.scale),
            };

            let node = crate::scene2::Node {
                transform,
                components: map_components(&node.components, parent, graph, renderer),
            };
            graph.append(Some(parent), todo!());
        }
    }
}

fn map_components(
    components: &[Component],
    parent: Key,
    graph: &mut SceneGraph,
    renderer: &mut Option<&mut Renderer>,
) -> Vec<crate::scene2::Component> {
    let mut comps = Vec::new();

    for comp in components {
        match comp {
            Component::DirectionalLight(light) => {
                let c =
                    crate::scene2::Component::DirectionalLight(crate::scene2::DirectionalLight {
                        color: Color::from_rgb(light.color),
                        illuminance: light.illuminance,
                    });

                comps.push(c);
            }
            Component::PointLight(light) => {
                let c = crate::scene2::Component::PointLight(crate::scene2::PointLight {
                    color: Color::from_rgb(light.color),
                    intensity: light.intensity,
                    radius: light.radius,
                });

                comps.push(c);
            }
            Component::SpotLight(light) => {
                let c = crate::scene2::Component::SpotLight(crate::scene2::SpotLight {
                    color: Color::from_rgb(light.color),
                    intensity: light.intensity,
                    radius: light.radius,
                    inner_cutoff: light.inner_cutoff,
                    outer_cutoff: light.outer_cutoff,
                });

                comps.push(c);
            }
            Component::MeshInstance(instance) => {
                if let Some(renderer) = renderer {
                    load_mesh_instance(&instance.path, parent, graph, renderer);
                }
            }
            Component::Collider(collider) => {
                let c = crate::scene2::Component::Collider(match collider.shape {
                    ColliderShape::Cuboid(cuboid) => crate::scene2::Collider {
                        mass: 1.0,
                        friction: 1.0,
                        restitution: 0.5,
                        shape: crate::scene2::ColliderShape::Cuboid(crate::scene2::Cuboid {
                            hx: cuboid.hx,
                            hy: cuboid.hy,
                            hz: cuboid.hz,
                        }),
                    },
                });

                comps.push(c);
            }
        }
    }

    comps
}

fn load_mesh_instance(path: &str, parent: Key, graph: &mut SceneGraph, renderer: &mut Renderer) {
    let mut file = File::open(path).unwrap();

    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();

    let mut decoder = game_gltf::GltfDecoder::new(&buf).unwrap();
    while let Some(src) = decoder.pop_source() {
        let mut file = File::open(&src).unwrap();

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();

        decoder.push_source(src, buf);
    }

    let data = decoder.finish().unwrap();

    let scene = data.default_scene().unwrap();

    let mut meshes = HashMap::new();
    let mut materials = HashMap::new();
    let mut images = HashMap::new();

    for (index, mesh) in data.meshes.clone() {
        let id = renderer.meshes.insert(convert_mesh(mesh));
        meshes.insert(index, id);
    }

    for (index, image) in data.images.clone() {
        let id = renderer.images.insert(image);
        images.insert(index, id);
    }

    for (index, material) in data.materials.clone() {
        let id = renderer
            .materials
            .insert(create_material(material, &mut images));
        materials.insert(index, id);
    }

    let mut parents = HashMap::new();
    for (key, node) in scene.nodes.iter() {
        if scene.nodes.parent(key).is_some() {
            continue;
        }

        let new_key = match (node.mesh, node.material) {
            (Some(mesh), Some(material)) => {
                let mesh = *meshes.get(&mesh).unwrap();
                let material = *materials.get(&material).unwrap();

                graph.append(Some(parent), todo!())
            }
            _ => graph.append(Some(parent), todo!()),
        };

        if let Some(children) = scene.nodes.children(key) {
            for (child, _) in children {
                parents.insert(child, new_key);
            }
        }
    }

    while !parents.is_empty() {
        for (child, parent) in parents.clone().iter() {
            let node = scene.nodes.get(*child).unwrap();
            parents.remove(child);

            let new_key = match (node.mesh, node.material) {
                (Some(mesh), Some(material)) => {
                    let mesh = *meshes.get(&mesh).unwrap();
                    let material = *materials.get(&material).unwrap();

                    graph.append(Some(*parent), todo!())
                }
                _ => graph.append(Some(*parent), todo!()),
            };

            if let Some(children) = scene.nodes.children(*child) {
                for (child, _) in children {
                    parents.insert(child, new_key);
                }
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub parent: Option<usize>,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub components: Vec<Component>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Component {
    MeshInstance(MeshInstance),
    DirectionalLight(DirectionalLight),
    PointLight(PointLight),
    SpotLight(SpotLight),
    Collider(Collider),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshInstance {
    pub path: String,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectionalLight {
    pub color: [f32; 3],
    pub illuminance: f32,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointLight {
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpotLight {
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
    pub inner_cutoff: f32,
    pub outer_cutoff: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Collider {
    pub shape: ColliderShape,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ColliderShape {
    Cuboid(Cuboid),
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cuboid {
    pub hx: f32,
    pub hy: f32,
    pub hz: f32,
}

pub fn from_slice(buf: &[u8]) -> Result<SceneRoot, Box<dyn std::error::Error>> {
    let root: SceneRoot = serde_json::from_slice(buf)?;

    Ok(root)
}

fn convert_mesh(input: GltfMesh) -> Mesh {
    let mut mesh = Mesh::new();

    mesh.set_positions(input.positions);
    mesh.set_normals(input.normals);
    mesh.set_uvs(input.uvs);
    mesh.set_indices(Indices::U32(input.indices));

    // FIXME: Still need to figure out where exactly we want to
    // generate tangents. This used to be in the gltf crate, but
    // we don't want it to depend on the rendering crate, so I guess
    // we'll just do it here for now.
    if input.tangents.is_empty() {
        mesh.compute_tangents();
    } else {
        mesh.set_tangents(input.tangents);
    }

    mesh
}

fn create_material(
    material: GltfMaterial,
    images: &mut HashMap<TextureIndex, ImageId>,
) -> PbrMaterial {
    let base_color_texture = material.base_color_texture.map(|index| images[&index]);
    let normal_texture = material.normal_texture.map(|index| images[&index]);
    let metallic_roughness_texture = material
        .metallic_roughness_texture
        .map(|index| images[&index]);

    PbrMaterial {
        alpha_mode: convert_alpha_mode(material.alpha_mode),
        base_color: material.base_color,
        base_color_texture,
        normal_texture,
        roughness: material.roughness,
        metallic: material.metallic,
        metallic_roughness_texture,
        ..Default::default()
    }
}

fn convert_alpha_mode(value: game_gltf::AlphaMode) -> AlphaMode {
    match value {
        game_gltf::AlphaMode::Opaque => AlphaMode::Opaque,
        game_gltf::AlphaMode::Mask => AlphaMode::Mask,
        game_gltf::AlphaMode::Blend => AlphaMode::Blend,
    }
}
