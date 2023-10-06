use game_model::material::Material;
use game_model::Model;
use game_render::color::Color;
use game_render::mesh::{Indices, Mesh};
use game_render::pbr::AlphaMode;
use game_render::texture::{Image, TextureFormat};
use game_tracing::trace_span;
use glam::UVec2;

use crate::loader::LoadScene;
use crate::scene::{Node, NodeBody, ObjectNode, Scene};

impl LoadScene for Model {
    fn load(self) -> Scene {
        let _span = trace_span!("Model::load").entered();

        let mut scene = Scene::default();

        for mesh in self.meshes {
            let positions = self.buffers[mesh.positions as usize].as_positions();
            let normals = self.buffers[mesh.normals as usize].as_normals();
            let tangents = self.buffers[mesh.tangents as usize].as_tangents();
            let uvs = self.buffers[mesh.uvs as usize].as_uvs();
            let indices = self.buffers[mesh.indices as usize].as_indices();

            let mut mesh = Mesh::new();
            mesh.set_positions(positions.into_iter().map(|pos| pos.to_array()).collect());
            mesh.set_normals(normals.into_iter().map(|norm| norm.to_array()).collect());
            mesh.set_tangents(tangents.to_vec());
            mesh.set_uvs(uvs.into_iter().map(|uv| uv.to_array()).collect());
            mesh.set_indices(Indices::U32(indices.to_vec()));

            scene.meshes.push(mesh);
        }

        for texture in self.textures {
            let image = Image::new(
                UVec2::new(texture.width, texture.height),
                convert_texture_format(texture.format),
                texture.bytes,
            );

            scene.images.push(image);
        }

        for material in self.materials {
            let material = create_material(material);
            scene.materials.push(material);
        }

        for node in self.nodes {
            let mesh = node.mesh as usize;
            let material = node.material as usize;

            scene.nodes.push(Node {
                transform: node.transform,
                body: NodeBody::Object(ObjectNode { mesh, material }),
            });
        }

        scene
    }
}

fn create_material(material: Material) -> crate::scene::Material {
    let material = match material {
        Material::MetallicRoughness(mat) => mat,
    };

    let base_color = Color(material.base_color.map(|channel| channel as f32 / 255.0));

    let albedo_texture = material.albedo_texture.map(|index| index as usize);

    let normal_texture = material.normal_texture.map(|index| index as usize);

    let metallic_roughness_texture = material
        .metallic_roughness_texture
        .map(|index| index as usize);

    crate::scene::Material {
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
