use std::collections::HashMap;

use game_common::components::rendering::Color;
use game_common::components::transform::Transform;
use game_core::hierarchy::Hierarchy;
use game_render::entities::Object;
use game_render::mesh::Mesh;
use game_render::pbr::{AlphaMode, PbrMaterial};
use game_render::texture::Image;
use game_render::{shape, Renderer};
use game_tracing::trace_span;

use crate::scene2::{Component, Key, MeshInstance, SceneGraph};

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub nodes: Hierarchy<Node>,
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
    Empty,
    Object(ObjectNode),
    DirectionalLight(crate::scene2::DirectionalLight),
    PointLight(crate::scene2::PointLight),
    SpotLight(crate::scene2::SpotLight),
}

#[derive(Copy, Clone, Debug)]
pub struct ObjectNode {
    pub mesh: usize,
    pub material: usize,
}

impl Scene {
    pub(crate) fn spawn(
        self,
        renderer: &mut Option<&mut Renderer>,
        parent: Key,
        graph: &mut SceneGraph,
    ) {
        let _span = trace_span!("Scene::spawn").entered();

        let Some(renderer) = renderer else {
            return;
        };

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

        let mut parents = HashMap::new();

        for (key, node) in self.nodes.iter() {
            if self.nodes.parent(key).is_some() {
                continue;
            }

            let entity = graph.append(
                Some(parent),
                crate::scene2::Node {
                    transform: node.transform,
                    components: match node.body {
                        NodeBody::Empty => vec![],
                        NodeBody::Object(obj) => vec![Component::MeshInstance(MeshInstance {
                            mesh: meshes[obj.mesh],
                            material: materials[obj.material],
                        })],
                        NodeBody::DirectionalLight(light) => {
                            vec![Component::DirectionalLight(light)]
                        }
                        NodeBody::PointLight(light) => vec![Component::PointLight(light)],
                        NodeBody::SpotLight(light) => vec![Component::SpotLight(light)],
                    },
                },
            );
            if let Some(children) = self.nodes.children(key) {
                for (child, _) in children {
                    parents.insert(child, entity);
                }
            }
        }

        while !parents.is_empty() {
            for (child, parent) in parents.clone().iter() {
                let node = self.nodes.get(*child).unwrap();
                parents.remove(child);

                let entity = graph.append(
                    Some(*parent),
                    crate::scene2::Node {
                        transform: node.transform,
                        components: match node.body {
                            NodeBody::Empty => vec![],
                            NodeBody::Object(obj) => vec![Component::MeshInstance(MeshInstance {
                                mesh: meshes[obj.mesh],
                                material: materials[obj.material],
                            })],
                            NodeBody::DirectionalLight(light) => {
                                vec![Component::DirectionalLight(light)]
                            }
                            NodeBody::PointLight(light) => vec![Component::PointLight(light)],
                            NodeBody::SpotLight(light) => vec![Component::SpotLight(light)],
                        },
                    },
                );
                if let Some(children) = self.nodes.children(*child) {
                    for (child, _) in children {
                        parents.insert(child, entity);
                    }
                }

                children.push(entity);
            }
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
