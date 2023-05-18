use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use bytes::Buf;
use game_render::mesh::Mesh;
use glam::Vec3;
use gltf::accessor::DataType;
use gltf::accessor::Dimensions;
use gltf::buffer::Source;
use gltf::{Accessor, Gltf, Semantic};
use indexmap::IndexMap;

pub struct GltfData {
    pub gltf: Gltf,
    pub buffers: IndexMap<String, Vec<u8>>,
}

impl GltfData {
    pub fn open<P>(path: P) -> Result<Self, ()>
    where
        P: AsRef<Path>,
    {
        let file = Gltf::open(path).unwrap();

        let mut buffers = IndexMap::new();
        for buffer in file.buffers() {
            match buffer.source() {
                Source::Bin => todo!(),
                Source::Uri(uri) => {
                    let mut file = File::open(uri).unwrap();

                    let mut buf = Vec::new();
                    file.read_to_end(&mut buf).unwrap();

                    buffers.insert(uri.to_owned(), buf);
                }
            }
        }

        Ok(Self {
            gltf: file,
            buffers,
        })
    }
}

pub fn load_mesh(gltf: GltfData) -> Mesh {
    let scene = gltf.gltf.scenes().nth(0).unwrap();

    for node in scene.nodes() {
        node.transform();

        if let Some(mesh) = node.mesh() {}
    }

    for mesh in gltf.gltf.meshes() {
        for primitive in mesh.primitives() {
            let attrs = primitive.attributes();

            for (semantic, accessor) in attrs {
                match semantic {
                    Semantic::Positions => {
                        let data_type = accessor.data_type();

                        accessor.dimensions();
                    }
                    _ => todo!(),
                }
            }
        }
    }

    todo!()
}

impl GltfData {
    fn buffer(&self, uri: &str) -> &[u8] {
        let buffer = self.buffers.get(uri).unwrap();
        &buffer
    }
}

fn load_positions(data: &GltfData, acessor: &Accessor) -> Vec<Vec3> {
    let data_type = acessor.data_type();
    let dimensions = acessor.dimensions();

    assert_eq!(data_type, DataType::F32);
    assert_eq!(dimensions, Dimensions::Vec3);

    let view = acessor.view().unwrap();
    let buffer = view.buffer();

    let mut buf =
        &data.buffer(buffer.name().unwrap())[view.offset()..view.offset() + view.length()];

    let mut vec = vec![];
    while buf.len() != 0 {
        let x = buf.get_f32_le();
        let y = buf.get_f32_le();
        let z = buf.get_f32_le();

        vec.push(Vec3 { x, y, z });
    }

    vec
}
