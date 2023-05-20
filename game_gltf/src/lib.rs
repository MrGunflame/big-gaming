mod mime;
mod uri;

use std::fs::File;
use std::io::Read;
use std::path::Path;

use base64::alphabet::STANDARD;
use base64::engine::GeneralPurpose;
use base64::engine::GeneralPurposeConfig;
use base64::Engine;
use bytes::Buf;
use game_render::mesh::Indices;
use game_render::mesh::Mesh;
use game_render::pbr::AlphaMode;
use game_render::pbr::PbrMaterial;
use gltf::accessor::DataType;
use gltf::accessor::Dimensions;
use gltf::buffer::Source;
use gltf::Image;
use gltf::Material;
use gltf::{Accessor, Gltf, Semantic};
use indexmap::IndexMap;
use mime::MimeType;
use uri::Uri;

use gltf::image::Source as ImageSource;

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
                    if let Some(data) = uri.strip_prefix("data:application/octet-stream;base64,") {
                        let engine = GeneralPurpose::new(&STANDARD, GeneralPurposeConfig::new());
                        let buf = engine.decode(data).unwrap();

                        buffers.insert(uri.to_owned(), buf);
                    } else {
                        let mut path = path.clone();
                        path.push(uri);

                        let mut file = File::open(path.as_path()).unwrap();

                        let mut buf = Vec::new();
                        file.read_to_end(&mut buf).unwrap();

                        buffers.insert(uri.to_owned(), buf);
                    }
                }
            }
        }

        for image in file.images() {
            if let ImageSource::Uri { uri, mime_type } = image.source() {
                let mut path = path.clone();
                path.push(uri);

                let mut file = File::open(path.as_path()).unwrap();

                let mut buf = Vec::new();
                file.read_to_end(&mut buf).unwrap();

                buffers.insert(uri.to_owned(), buf);
            }
        }

        Ok(Self {
            gltf: file,
            buffers,
        })
    }

    pub fn meshes(&self) -> Vec<(Mesh, PbrMaterial)> {
        let mut meshes = Vec::new();

        for mesh in self.gltf.meshes() {
            for primitive in mesh.primitives() {
                let mut out_mesh = Mesh::new();

                let attrs = primitive.attributes();

                for (semantic, accessor) in attrs {
                    match semantic {
                        Semantic::Positions => {
                            let mut positions = vec![];
                            self.load_positions(&accessor, &mut positions);
                            out_mesh.set_positions(positions);
                        }
                        Semantic::Normals => {
                            let mut normals = vec![];
                            self.load_normals(&accessor, &mut normals);
                            out_mesh.set_normals(normals);
                        }
                        Semantic::TexCoords(index) => {
                            if index != 0 {
                                panic!("multiple texture coordinates not yet supported");
                            }

                            let mut uvs = vec![];
                            self.load_uvs(&accessor, &mut uvs);
                            out_mesh.set_uvs(uvs);
                        }
                        _ => (),
                    }
                }

                if let Some(accessor) = primitive.indices() {
                    let mut indices = Indices::U16(vec![]);
                    self.load_indices(&accessor, &mut indices);
                    out_mesh.set_indices(indices);
                }

                let material = self.load_material(primitive.material());

                meshes.push((out_mesh, material));
            }
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

    fn load_normals(&self, accessor: &Accessor, normals: &mut Vec<[f32; 3]>) {
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

            normals.push([x, y, z]);
        }
    }

    fn load_uvs(&self, accessor: &Accessor, uvs: &mut Vec<[f32; 2]>) {
        let data_type = accessor.data_type();
        let dimensions = accessor.dimensions();

        assert_eq!(data_type, DataType::F32);
        assert_eq!(dimensions, Dimensions::Vec2);

        let view = accessor.view().unwrap();
        let buffer = view.buffer();

        let mut buf = self.buffer(buffer.source(), view.offset(), view.length());

        while buf.len() != 0 {
            let x = buf.get_f32_le();
            let y = buf.get_f32_le();

            uvs.push([x, y]);
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

    fn load_material(&self, material: Material<'_>) -> PbrMaterial {
        let alpha_mode = convert_alpha_mode(material.alpha_mode());

        let pbr = material.pbr_metallic_roughness();

        let base_color = pbr.base_color_factor();

        let base_color_texture = if let Some(info) = pbr.base_color_texture() {
            let image = info.texture().source();

            let buf = self.load_image(image);
            Some(buf.to_vec())
        } else {
            None
        };

        let roughness = pbr.roughness_factor();
        let metallic = pbr.metallic_factor();

        let metallic_roughness_texture = if let Some(info) = pbr.metallic_roughness_texture() {
            let image = info.texture().source();

            let buf = self.load_image(image);
            Some(buf.to_vec())
        } else {
            None
        };

        PbrMaterial {
            alpha_mode,
            base_color,
            base_color_texture,
            roughness,
            metallic,
            metallic_roughness_texture,
        }
    }

    fn load_image(&self, image: Image<'_>) -> &[u8] {
        match image.source() {
            ImageSource::View { view, mime_type } => {
                // Validate mime type
                mime_type.parse::<MimeType>().unwrap();

                self.buffer(view.buffer().source(), view.offset(), view.length())
            }
            ImageSource::Uri { uri, mime_type } => {
                // Validate mime type
                mime_type.unwrap().parse::<MimeType>().unwrap();

                let len = self.buffers.get(uri).unwrap().len();
                self.buffer(Source::Uri(uri), 0, len)
            }
        }
    }
}

fn convert_alpha_mode(value: gltf::material::AlphaMode) -> AlphaMode {
    match value {
        gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
        gltf::material::AlphaMode::Mask => AlphaMode::Mask,
        gltf::material::AlphaMode::Blend => AlphaMode::Blend,
    }
}
