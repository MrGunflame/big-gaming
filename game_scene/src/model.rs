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
        mesh.set_uvs(
            node.vertices
                .uvs
                .into_iter()
                .map(|vec| vec.to_array())
                .collect(),
        );

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

    let albedo_texture = material.albedo_texture.map(|index| {
        let texture = &textures[index as usize];

        images.insert(Image::new(
            UVec2::new(texture.width, texture.height),
            convert_texture_format(texture.format),
            texture.bytes.clone(),
        ))
    });

    let normal_texture = material.normal_texture.map(|index| {
        let texture = &textures[index as usize];

        images.insert(Image::new(
            UVec2::new(texture.width, texture.height),
            convert_texture_format(texture.format),
            texture.bytes.clone(),
        ))
    });

    let metallic_roughness_texture = material.metallic_roughness_texture.map(|index| {
        let texture = &textures[index as usize];

        images.insert(Image::new(
            UVec2::new(texture.width, texture.height),
            convert_texture_format(texture.format),
            texture.bytes.clone(),
        ))
    });

    PbrMaterial {
        alpha_mode: AlphaMode::Opaque,
        base_color,
        base_color_texture: albedo_texture,
        normal_texture: normal_texture,
        metallic_roughness_texture: metallic_roughness_texture,
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
