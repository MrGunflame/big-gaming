use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use clap::Parser;
use game_gltf::GltfData;
use game_model::compression::CompressionScheme;
use game_model::material::{Material, MetallicRoughnessMaterial};
use game_model::mesh::Mesh;
use game_model::textures::{Texture, TextureFormat};
use game_model::vertex::Vertices;
use game_model::{Encode, Header, Model};

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,
    #[arg(short, long)]
    output: PathBuf,
}

fn main() {
    let args = Args::parse();

    let gltf = load_gltf(args.input).unwrap();

    let model = convert_gltf_to_model(gltf);

    let mut buf = Vec::new();
    model.encode(&mut buf);

    let mut file = File::create(args.output).unwrap();
    file.write_all(&buf).unwrap();
}

fn load_gltf(path: impl AsRef<Path>) -> Result<GltfData, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let mut data = GltfData::new(&buf)?;

    while let Some(path) = data.queue.pop() {
        let mut file = File::open(&path)?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        data.insert(path, buf);
    }

    Ok(data.create())
}

fn convert_gltf_to_model(gltf: GltfData) -> Model {
    let mut meshes = Vec::new();

    for scene in gltf.scenes().unwrap() {
        for node in scene.nodes {
            if !node.children.is_empty() {
                panic!("nested nodes are not yet supported");
            }

            let Some(mesh) = node.mesh else {
                continue;
            };

            for primitive in mesh.primitives {
                let mut vertices = Vertices::default();
                vertices.positions = primitive
                    .mesh
                    .positions()
                    .iter()
                    .map(|&s| s.into())
                    .collect();
                vertices.normals = primitive.mesh.normals().iter().map(|&s| s.into()).collect();
                vertices.tangents = primitive
                    .mesh
                    .tangents()
                    .iter()
                    .map(|&s| s.into())
                    .collect();
                vertices.uvs = primitive.mesh.uvs().iter().map(|&s| s.into()).collect();
                vertices.indices = primitive.mesh.indicies().unwrap().into_u32();

                let mut albedo_texture = None;
                let mut normal_texture = None;
                let mut metallic_roughness_texture = None;

                let mut textures = vec![];
                if let Some(buf) = primitive.material.base_color_texture {
                    albedo_texture = Some(textures.len() as u16);

                    textures.push(create_texture(&buf, TextureFormat::Rgba8UnormSrgb));
                }

                if let Some(buf) = primitive.material.normal_texture {
                    normal_texture = Some(textures.len() as u16);

                    textures.push(create_texture(&buf, TextureFormat::Rgba8Unorm));
                }

                if let Some(buf) = primitive.material.metallic_roughness_texture {
                    metallic_roughness_texture = Some(textures.len() as u16);

                    textures.push(create_texture(&buf, TextureFormat::Rgba8UnormSrgb));
                }

                meshes.push(Mesh {
                    transform: node.transform,
                    vertices,
                    material: Material::MetallicRoughness(MetallicRoughnessMaterial {
                        base_color: primitive.material.base_color.0.map(|v| (v * 255.0) as u8),
                        metallic: (primitive.material.metallic * 255.0) as u8,
                        roughness: (primitive.material.roughness * 255.0) as u8,
                        albedo_texture,
                        normal_texture,
                        metallic_roughness_texture,
                    }),
                    textures: vec![],
                });
            }
        }
    }

    Model {
        header: Header {
            version: 0,
            compression: CompressionScheme::None,
        },
        meshes,
    }
}

fn create_texture(bytes: &[u8], format: TextureFormat) -> Texture {
    let img = image::load_from_memory(bytes).unwrap().into_rgba8();

    Texture {
        format,
        width: img.width(),
        height: img.height(),
        bytes: img.into_raw(),
    }
}
