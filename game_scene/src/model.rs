use game_asset::Assets;
use game_common::components::transform::Transform;
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

    let mut data_meshes = Vec::new();
    for mesh in data.meshes {
        let positions = data.buffers[mesh.positions as usize].as_positions();
        let normals = data.buffers[mesh.normals as usize].as_normals();
        let tangents = data.buffers[mesh.tangents as usize].as_tangents();
        let uvs = data.buffers[mesh.uvs as usize].as_uvs();
        let indices = data.buffers[mesh.indices as usize].as_indices();

        let mut mesh = Mesh::new();
        mesh.set_positions(positions.into_iter().map(|pos| pos.to_array()).collect());
        mesh.set_normals(normals.into_iter().map(|norm| norm.to_array()).collect());
        mesh.set_tangents(tangents.to_vec());
        mesh.set_uvs(uvs.into_iter().map(|uv| uv.to_array()).collect());
        mesh.set_indices(Indices::U32(indices.to_vec()));

        data_meshes.push(meshes.insert(mesh));
    }

    let mut data_materials = Vec::new();
    for mat in data.materials {
        let material = create_material(&data.textures, mat, images);
        data_materials.push(materials.insert(material));
    }

    for node in data.nodes {
        let mesh = data_meshes[node.mesh as usize].clone();
        let material = data_materials[node.material as usize].clone();

        nodes.push(Node {
            mesh,
            material,
            transform: node.transform,
        });
    }

    Scene {
        transform: Transform::default(),
        nodes,
    }
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
