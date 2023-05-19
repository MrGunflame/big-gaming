mod uri;

use std::fs::File;
use std::io::Read;
use std::path::Path;

use bytes::Buf;
use game_render::mesh::Indices;
use game_render::mesh::Mesh;
use gltf::accessor::DataType;
use gltf::accessor::Dimensions;
use gltf::buffer::Source;
use gltf::{Accessor, Gltf, Semantic};
use indexmap::IndexMap;
use uri::Uri;

pub struct GltfData {
    pub gltf: Gltf,
    pub buffers: IndexMap<String, Vec<u8>>,
}

impl GltfData {
    pub fn open<P>(path: P) -> Result<Self, ()>
    where
        P: AsRef<Path>,
    {
        let path = Uri::from(path);

        let file = Gltf::open(path.as_path()).unwrap();

        let mut buffers = IndexMap::new();
        for buffer in file.buffers() {
            match buffer.source() {
                Source::Bin => {
                    buffers.insert(String::from(""), file.blob.clone().unwrap());
                }
                Source::Uri(uri) => {
                    let mut path = path.clone();
                    path.push(uri);

                    let mut file = File::open(path.as_path()).unwrap();

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

    pub fn meshes(&self) -> Vec<Mesh> {
        let mut meshes = Vec::new();

        for mesh in self.gltf.meshes() {
            let mut out_mesh = Mesh::new();

            for primitive in mesh.primitives() {
                let attrs = primitive.attributes();

                for (semantic, accessor) in attrs {
                    match semantic {
                        Semantic::Positions => {
                            let mut positions = vec![];
                            self.load_positions(&accessor, &mut positions);
                            out_mesh.set_positions(positions);
                        }
                        _ => (),
                    }
                }

                if let Some(accessor) = primitive.indices() {
                    let mut indices = Indices::U16(vec![]);
                    self.load_indices(&accessor, &mut indices);
                    out_mesh.set_indices(indices);
                }
            }

            meshes.push(out_mesh);
        }

        meshes
    }

    fn buffer(&self, source: Source, offset: usize, length: usize) -> &[u8] {
        let buf = match source {
            Source::Bin => {
                let (_, buf) = self.buffers.first().unwrap();
                buf
            }
            Source::Uri(uri) => {
                let buf = self.buffers.get(uri).unwrap();
                buf
            }
        };

        &buf[offset..offset + length]
    }

    fn load_positions(&self, accessor: &Accessor, positions: &mut Vec<[f32; 3]>) {
        let data_type = accessor.data_type();
        let dimensions = accessor.dimensions();

        assert_eq!(data_type, DataType::F32);
        assert_eq!(dimensions, Dimensions::Vec3);

        let view = accessor.view().unwrap();
        let buffer = view.buffer();

        let mut buf = self.buffer(buffer.source(), view.offset(), view.length());

        while buf.len() != 0 {
            let x = buf.get_f32_le();
            let y = buf.get_f32_le();
            let z = buf.get_f32_le();

            positions.push([x, y, z]);
        }
    }

    fn load_indices(&self, accessor: &Accessor, indices: &mut Indices) {
        let data_type = accessor.data_type();
        let dimensions = accessor.dimensions();

        assert_eq!(dimensions, Dimensions::Scalar);

        let view = accessor.view().unwrap();
        let buffer = view.buffer();

        let mut buf = self.buffer(buffer.source(), view.offset(), view.length());

        match data_type {
            DataType::U16 => {
                let mut out = vec![];

                while buf.len() != 0 {
                    let val = buf.get_u16_le();
                    out.push(val);
                }

                *indices = Indices::U16(out);
            }
            DataType::U32 => {
                let mut out = vec![];

                while buf.len() != 0 {
                    let val = buf.get_u32_le();
                    out.push(val);
                }

                *indices = Indices::U32(out);
            }
            _ => todo!(),
        }
    }
}
