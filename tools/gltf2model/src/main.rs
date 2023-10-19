mod ir;

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use game_gltf::GltfData;
use game_model::buffer::Buffer;
use game_model::compression::CompressionScheme;
use game_model::material::{Material, MetallicRoughnessMaterial};
use game_model::mesh::Mesh;
use game_model::textures::{Texture, TextureFormat};
use game_model::{Encode, Header, Model, Node};
use ir::ModelIr;

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

    let ir = convert_gltf_to_ir(gltf);
    let model = convert_ir_to_model(ir);

    let mut buf = Vec::new();
    model.encode(&mut buf);

    let mut file = File::create(args.output).unwrap();
    file.write_all(&buf).unwrap();
}

fn load_gltf(path: impl AsRef<Path>) -> Result<GltfData, Box<dyn std::error::Error>> {
    let data = GltfData::from_file(path)?;
    Ok(data)
}

fn convert_gltf_to_ir(gltf: GltfData) -> ModelIr {
    let mut ir = ModelIr::default();

    for scene in gltf.scenes {
        for (key, node) in scene.nodes.iter() {
            if !scene.nodes.parent(key).is_some() {
                panic!("nested nodes are not yet supported");
            }

            let (Some(mesh), Some(material)) = (node.mesh, node.material) else {
                continue;
            };

            let mesh = gltf.meshes[&mesh].clone();
            let material = gltf.materials[&material];

            let mut positions = Vec::new();
            for pos in &mesh.positions {
                positions.extend(bytemuck::bytes_of(pos));
            }

            let mut normals = Vec::new();
            for norm in &mesh.normals {
                normals.extend(bytemuck::bytes_of(norm));
            }

            let mut tangents = Vec::new();
            for tang in &mesh.tangents {
                tangents.extend(bytemuck::bytes_of(tang));
            }

            let mut uvs = Vec::new();
            for uv in &mesh.uvs {
                uvs.extend(bytemuck::bytes_of(uv));
            }

            let mut indices = Vec::new();
            for index in mesh.indices {
                indices.extend(bytemuck::bytes_of(&index));
            }

            let positions = ir.buffers.insert(Buffer { bytes: positions });
            let normals = ir.buffers.insert(Buffer { bytes: normals });
            let tangents = ir.buffers.insert(Buffer { bytes: tangents });
            let uvs = ir.buffers.insert(Buffer { bytes: uvs });
            let indices = ir.buffers.insert(Buffer { bytes: indices });

            let mut albedo_texture = None;
            let mut normal_texture = None;
            let mut metallic_roughness_texture = None;

            if let Some(tex_index) = material.base_color_texture {
                let texture = gltf.images[&tex_index].clone();

                let index = ir.textures.insert(create_texture(
                    texture.as_bytes(),
                    TextureFormat::Rgba8UnormSrgb,
                ));

                albedo_texture = Some(index);
            }

            if let Some(tex_index) = material.normal_texture {
                let texture = gltf.images[&tex_index].clone();

                let index = ir.textures.insert(create_texture(
                    texture.as_bytes(),
                    TextureFormat::Rgba8Unorm,
                ));

                normal_texture = Some(index);
            }

            if let Some(tex_index) = material.metallic_roughness_texture {
                let texture = gltf.images[&tex_index].clone();

                let index = ir.textures.insert(create_texture(
                    texture.as_bytes(),
                    TextureFormat::Rgba8UnormSrgb,
                ));

                metallic_roughness_texture = Some(index);
            }

            let mesh = ir.meshes.insert(Mesh {
                positions: positions.0,
                normals: normals.0,
                tangents: tangents.0,
                indices: indices.0,
                uvs: uvs.0,
            });

            let material =
                ir.materials
                    .insert(Material::MetallicRoughness(MetallicRoughnessMaterial {
                        base_color: material.base_color.0.map(|v| (v * 255.0) as u8),
                        metallic: (material.metallic * 255.0) as u8,
                        roughness: (material.roughness * 255.0) as u8,
                        albedo_texture: albedo_texture.map(|s| s.0),
                        normal_texture: normal_texture.map(|s| s.0),
                        metallic_roughness_texture: metallic_roughness_texture.map(|s| s.0),
                    }));

            ir.nodes.push(Node {
                transform: node.transform,
                mesh: mesh.0,
                material: material.0,
            });
        }
    }

    ir
}

fn convert_ir_to_model(ir: ModelIr) -> Model {
    let mut nodes = Vec::new();
    let mut meshes = Vec::new();
    let mut materials = Vec::new();
    let mut buffers = Vec::new();
    let mut textures = Vec::new();

    for node in ir.nodes {
        nodes.push(node);
    }

    for mesh in ir.meshes.meshes {
        meshes.push(mesh);
    }

    for buffer in ir.buffers.buffers {
        buffers.push(buffer);
    }

    for texture in ir.textures.textures {
        textures.push(texture);
    }

    for material in ir.materials.materials {
        materials.push(material);
    }

    Model {
        header: Header {
            version: 0,
            compression: CompressionScheme::None,
        },
        nodes,
        materials,
        meshes,
        buffers,
        textures,
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
