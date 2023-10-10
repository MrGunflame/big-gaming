use game_gltf::{GltfData, GltfMaterial};
use game_render::texture::{Image, TextureFormat};
use game_tracing::trace_span;
use glam::UVec2;

use crate::loader::LoadScene;
use crate::scene::{Material, Node, NodeBody, ObjectNode, Scene};

impl LoadScene for GltfData {
    fn load(self) -> Scene {
        let _span = trace_span!("GltfData::load").entered();

        let mut scene = Scene::default();

        for node in self.scenes().unwrap().into_iter().nth(0).unwrap().nodes {
            if let Some(mesh) = node.mesh {
                for primitive in mesh.primitives {
                    let mesh = scene.meshes.len();
                    scene.meshes.push(primitive.mesh);
                    let material = scene.materials.len();
                    scene
                        .materials
                        .push(create_material(primitive.material, &mut scene.images));

                    scene.nodes.push(Node {
                        transform: node.transform,
                        body: NodeBody::Object(ObjectNode { mesh, material }),
                    });
                }
            }
        }

        // TODO: Children

        scene
    }
}

fn create_material(material: GltfMaterial, images: &mut Vec<Image>) -> Material {
    let base_color_texture = material.base_color_texture.map(|buf| {
        let index = images.len();
        let img = load_image(&buf, TextureFormat::Rgba8UnormSrgb);
        images.push(img);
        index
    });
    let normal_texture = material.normal_texture.map(|buf| {
        let index = images.len();
        let img = load_image(&buf, TextureFormat::Rgba8Unorm);
        images.push(img);
        index
    });
    let metallic_roughness_texture = material.metallic_roughness_texture.map(|buf| {
        let index = images.len();
        let img = load_image(&buf, TextureFormat::Rgba8Unorm);
        images.push(img);
        index
    });

    Material {
        alpha_mode: material.alpha_mode,
        base_color: material.base_color,
        base_color_texture,
        normal_texture,
        roughness: material.roughness,
        metallic: material.metallic,
        metallic_roughness_texture,
    }
}

fn load_image(buf: &[u8], format: TextureFormat) -> Image {
    let img = image::load_from_memory(&buf).unwrap().to_rgba8();
    Image::new(
        UVec2::new(img.width(), img.height()),
        format,
        img.into_raw(),
    )
}
