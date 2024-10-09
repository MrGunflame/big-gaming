use std::collections::HashMap;

use game_common::components::{Color, Transform};
use game_core::hierarchy::Hierarchy;
use game_render::entities::Object;
use game_render::mesh::Mesh;
use game_render::pbr::{AlphaMode, PbrMaterial};
use game_render::scene::RendererScene;
use game_render::shape;
use game_render::texture::Image;
use game_tracing::trace_span;

use crate::scene2::{Key, SceneResources, SpawnedScene};

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
    pub(crate) fn setup_materials(&mut self, renderer: &mut RendererScene<'_>) -> SceneResources {
        let meshes = self
            .meshes
            .drain(..)
            .map(|mesh| renderer.meshes.insert(mesh))
            .collect();

        let images = self
            .images
            .drain(..)
            .map(|image| renderer.images.insert(image))
            .collect::<Vec<_>>();

        let materials = self
            .materials
            .drain(..)
            .map(|material| {
                renderer.materials.insert({
                    PbrMaterial {
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
                    }
                })
            })
            .collect();

        SceneResources {
            meshes,
            materials,
            images,
        }
    }

    pub(crate) fn instantiate(
        &self,
        res: &SceneResources,
        renderer: &mut RendererScene<'_>,
    ) -> SpawnedScene {
        let _span = trace_span!("Scene::instantiate").entered();

        let mut spawned_scene = SpawnedScene::new();

        let mut children = Vec::new();

        let mut parents = HashMap::new();

        for (key, node) in self.nodes.iter() {
            if self.nodes.parent(key).is_some() {
                continue;
            }

            let entity = spawned_scene.append(None, node.clone());
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

                let entity = spawned_scene.append(Some(*parent), node.clone());
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
            renderer.scene.entities.objects.insert(Object {
                transform: Default::default(),
                mesh: renderer.meshes.insert(mesh.into()),
                material: renderer.materials.insert(PbrMaterial {
                    base_color: color,
                    ..Default::default()
                }),
            });
        }

        spawned_scene.compute_transform();

        for (key, node) in spawned_scene.nodes.iter() {
            match node.body {
                NodeBody::Empty => {}
                NodeBody::Object(object) => {
                    let mesh = res.meshes[object.mesh];
                    let material = res.materials[object.material];
                    let transform = *spawned_scene.global_transform.get(&Key(key)).unwrap();

                    let id = renderer.scene.entities.objects.insert(Object {
                        transform,
                        mesh,
                        material,
                    });

                    spawned_scene.entities.insert(Key(key), id);
                }
                _ => todo!(),
            }
        }

        spawned_scene
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
