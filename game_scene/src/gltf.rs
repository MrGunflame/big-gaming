use game_asset::Assets;
use game_gltf::{GltfData, GltfLoader, GltfMaterial};
use game_render::mesh::Mesh;
use game_render::pbr::PbrMaterial;
use game_render::texture::Images;

use crate::{Node, Scene};

#[derive(Clone, Debug)]
pub(crate) enum GltfState {
    Loading(GltfLoader),
    Ready(GltfData),
}

pub(crate) fn gltf_to_scene(
    data: GltfData,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<PbrMaterial>,
    images: &mut Images,
) -> Scene {
    let scenes = data.scenes().unwrap();
    let scene = scenes.into_iter().nth(0).unwrap();

    let mut nodes = Vec::new();

    for node in scene.nodes {
        if let Some(mesh) = node.mesh {
            for primitive in mesh.primitives {
                let mesh = meshes.insert(primitive.mesh);
                let material = materials.insert(create_material(primitive.material, images));

                nodes.push(Node {
                    mesh,
                    material,
                    transform: node.transform,
                });
            }
        }
    }

    // TODO: Children

    Scene { nodes }
}

fn create_material(material: GltfMaterial, images: &mut Images) -> PbrMaterial {
    let base_color_texture = material.base_color_texture.map(|buf| images.load(buf));
    let normal_texture = material.normal_texture.map(|buf| images.load(buf));
    let metallic_roughness_texture = material
        .metallic_roughness_texture
        .map(|buf| images.load(buf));

    PbrMaterial {
        alpha_mode: material.alpha_mode,
        base_color: material.base_color,
        base_color_texture,
        normal_texture,
        roughness: material.roughness,
        metallic: material.metallic,
        metallic_roughness_texture,
    }
}