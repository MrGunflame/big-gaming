use game_asset::Assets;
use game_model::material::Material;
use game_model::textures::Texture;
use game_model::Model;
use game_render::color::Color;
use game_render::mesh::{Indices, Mesh};
use game_render::pbr::{AlphaMode, PbrMaterial};
use game_render::texture::{Image, Images, TextureFormat};
use glam::UVec2;

use crate::{Node, Scene};

pub(crate) fn model_to_scene(
    data: Model,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<PbrMaterial>,
    images: &mut Images,
) -> Scene {
    let mut nodes = Vec::new();

    for node in data.meshes {
        let mut mesh = Mesh::new();
        mesh.set_positions(
            node.vertices
                .positions
                .into_iter()
                .map(|vec| vec.to_array())
                .collect(),
        );
        mesh.set_normals(
            node.vertices
                .normals
                .into_iter()
                .map(|vec| vec.to_array())
                .collect(),
        );
        mesh.set_tangents(node.vertices.tangents);
        mesh.set_indices(Indices::U32(node.vertices.indices));

        let mesh = meshes.insert(mesh);
        let material = materials.insert(create_material(&node.textures, node.material, images));

        nodes.push(Node {
            mesh,
            material,
            transform: node.transform,
        });
    }

    Scene { nodes }
}

fn create_material(textures: &[Texture], material: Material, images: &mut Images) -> PbrMaterial {
    let material = match material {
        Material::MetallicRoughness(mat) => mat,
    };

    let base_color = Color(material.base_color.map(|channel| channel as f32 / 255.0));

    let albedo_texture = &textures[material.albedo_texture as usize];
    let normal_texture = &textures[material.normal_texture as usize];
    let metallic_roughness_texture = &textures[material.metallic_roughness_texture as usize];

    let albedo_texture = images.insert(Image::new(
        UVec2::new(albedo_texture.width, albedo_texture.height),
        convert_texture_format(albedo_texture.format),
        albedo_texture.bytes.clone(),
    ));

    let normal_texture = images.insert(Image::new(
        UVec2::new(normal_texture.width, normal_texture.height),
        convert_texture_format(normal_texture.format),
        normal_texture.bytes.clone(),
    ));

    let metallic_roughness_texture = images.insert(Image::new(
        UVec2::new(
            metallic_roughness_texture.width,
            metallic_roughness_texture.height,
        ),
        convert_texture_format(metallic_roughness_texture.format),
        metallic_roughness_texture.bytes.clone(),
    ));

    PbrMaterial {
        alpha_mode: AlphaMode::Opaque,
        base_color,
        base_color_texture: Some(albedo_texture),
        normal_texture: Some(normal_texture),
        metallic_roughness_texture: Some(metallic_roughness_texture),
        roughness: material.roughness as f32 / 255.0,
        metallic: material.metallic as f32 / 255.0,
    }
}

fn convert_texture_format(format: game_model::textures::TextureFormat) -> TextureFormat {
    use game_model::textures::TextureFormat::*;

    match format {
        Rgba8UnormSrgb => TextureFormat::Rgba8UnormSrgb,
        Rgba8Unorm => TextureFormat::Rgba8Unorm,
    }
}
