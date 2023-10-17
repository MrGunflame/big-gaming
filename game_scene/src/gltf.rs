use std::collections::HashMap;

use game_gltf::types::{GltfMaterial, GltfMesh, TextureIndex};
use game_gltf::GltfData;
use game_render::mesh::{Indices, Mesh};
use game_render::pbr::AlphaMode;
use game_render::texture::Image;
use game_tracing::trace_span;
use glam::{Vec2, Vec3};

use crate::loader::LoadScene;
use crate::scene::{Material, Node, NodeBody, ObjectNode, Scene};

impl LoadScene for GltfData {
    fn load(mut self) -> Scene {
        let _span = trace_span!("GltfData::load").entered();

        let mut scene = Scene::default();

        let mut mesh_indices = HashMap::new();
        let mut material_indices = HashMap::new();

        for (key, node) in self
            .scenes
            .iter()
            .enumerate()
            .filter_map(|(index, scene)| {
                if Some(index) == self.default_scene {
                    Some(scene)
                } else {
                    None
                }
            })
            .nth(0)
            .unwrap()
            .nodes
            .iter()
        {
            // Mesh and material always come together as one or nothing.
            // There is no such thing as a mesh without material.
            if let (Some(mesh), Some(material)) = (node.mesh, node.material) {
                let mesh_index = mesh_indices.entry(mesh).or_insert_with(|| {
                    let mesh_index = scene.meshes.len();
                    scene
                        .meshes
                        .insert(mesh_index, convert_mesh(self.meshes[&mesh].clone()));

                    mesh_index
                });

                let material_index = material_indices.entry(material).or_insert_with(|| {
                    let material_index = scene.materials.len();
                    scene.materials.push(create_material(
                        self.materials[&material],
                        &mut scene.images,
                        &mut self.images,
                    ));
                    material_index
                });

                scene.nodes.push(Node {
                    transform: node.transform,
                    body: NodeBody::Object(ObjectNode {
                        mesh: *mesh_index,
                        material: *material_index,
                    }),
                });
            }
        }

        // TODO: Children

        scene
    }
}

fn create_material(
    material: GltfMaterial,
    images: &mut Vec<Image>,
    gltf_images: &mut HashMap<TextureIndex, Image>,
) -> Material {
    let base_color_texture = material.base_color_texture.map(|index| {
        let img_index = images.len();
        let img = gltf_images[&index].clone();
        images.push(img);
        img_index
    });
    let normal_texture = material.normal_texture.map(|index| {
        let img_index = images.len();
        let img = gltf_images[&index].clone();
        images.push(img);
        img_index
    });
    let metallic_roughness_texture = material.metallic_roughness_texture.map(|index| {
        let img_index = images.len();
        let img = gltf_images[&index].clone();
        images.push(img);
        img_index
    });

    Material {
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

fn convert_mesh(input: GltfMesh) -> Mesh {
    let mut mesh = Mesh::new();

    mesh.set_positions(input.positions.iter().map(Vec3::to_array).collect());
    mesh.set_normals(input.normals.iter().map(Vec3::to_array).collect());
    mesh.set_uvs(input.uvs.iter().map(Vec2::to_array).collect());
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

fn convert_alpha_mode(value: game_gltf::AlphaMode) -> AlphaMode {
    match value {
        game_gltf::AlphaMode::Opaque => AlphaMode::Opaque,
        game_gltf::AlphaMode::Mask => AlphaMode::Mask,
        game_gltf::AlphaMode::Blend => AlphaMode::Blend,
    }
}
