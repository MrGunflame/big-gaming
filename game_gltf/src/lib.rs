//! GLTF loader
//!
//!
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

mod accessor;
mod material;
mod mesh;
mod mime;
mod scene;

pub mod types;
pub mod uri;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::ops::Range;
use std::path::Path;

use accessor::{ItemReader, Normals, Positions, Tangents, Uvs};
use base64::alphabet::STANDARD;
use base64::engine::GeneralPurpose;
use base64::engine::GeneralPurposeConfig;
use base64::Engine;
use bytes::Buf;
use game_common::components::transform::Transform;
use game_core::hierarchy::Hierarchy;
use game_render::color::Color;
use game_render::texture::{Image, TextureFormat};
use game_tracing::trace_span;
use glam::{Quat, UVec2, Vec2, Vec3, Vec4};
use gltf::accessor::DataType;
use gltf::accessor::Dimensions;
use gltf::buffer::Source;
use gltf::mesh::Mode;
use gltf::Material;
use gltf::Node;
use gltf::{Accessor, Gltf, Semantic};
use mime::InvalidMimeType;
use mime::MimeType;
use serde_json::{Number, Value};
use thiserror::Error;
use types::{GltfMaterial, GltfMesh, GltfMeshMaterial, GltfNode, MaterialIndex, TextureIndex};
use uri::Uri;

pub use gltf::material::AlphaMode;
pub use scene::GltfScene;

use gltf::image::Source as ImageSource;

use crate::types::MeshIndex;

/// A GLTF file that is being loaded.
///
/// A (non-binary) GLTF file may reference to buffers from external URIs that have to be loaded
/// before the data in the GLTF file can be accessed.
///
/// The URIs that are required for this GLTF file are stored in `queue`.
// #[derive(Clone, Debug)]
// pub struct GltfLoader {
//     data: GltfData,
//     // FIXME: This could be &str since the string buffer
//     // is already in self.gltf.
//     pub queue: IndexSet<String>,
// }

// impl GltfLoader {
//     pub fn insert(&mut self, uri: String, buf: Vec<u8>) {
//         self.queue.remove(&uri);
//         self.data.buffers.insert(uri.to_owned(), buf.to_vec());
//     }

//     pub fn create(self) -> GltfData {
//         assert!(self.queue.is_empty());
//         self.create_unchecked()
//     }

//     pub fn create_unchecked(self) -> GltfData {
//         self.data
//     }
// }

const BASE64_PREFIX: &str = "data:application/octet-stream;base64,";

pub struct GltfDecoder {
    gltf: Gltf,
    buffers: HashMap<String, Vec<u8>>,
    external_sources: HashSet<String>,
}

impl GltfDecoder {
    pub fn new(slice: &[u8]) -> Result<Self, Error> {
        let _span = trace_span!("GltfDecoder::new").entered();

        let gltf = Gltf::from_slice(slice)?;

        let mut buffers = HashMap::new();
        let mut external_sources = HashSet::new();

        for buffer in gltf.buffers() {
            match buffer.source() {
                Source::Bin => {
                    buffers.insert(String::from(""), gltf.blob.clone().unwrap());
                }
                Source::Uri(uri) => {
                    if let Some(data) = uri.strip_prefix(BASE64_PREFIX) {
                        let engine = GeneralPurpose::new(&STANDARD, GeneralPurposeConfig::new());
                        let buf = engine.decode(data)?;

                        buffers.insert(uri.to_owned(), buf);
                    } else {
                        external_sources.insert(uri.to_owned());
                    }
                }
            }
        }

        for image in gltf.images() {
            if let ImageSource::Uri { uri, mime_type } = image.source() {
                // Validate the mime type.
                if let Some(mime_type) = mime_type {
                    let mime_type = mime_type.parse::<MimeType>()?;

                    if !mime_type.is_image() {
                        return Err(Error::NoImage(mime_type));
                    }
                }

                external_sources.insert(uri.to_owned());
            }
        }

        Ok(Self {
            gltf,
            buffers,
            external_sources,
        })
    }

    pub fn pop_source(&mut self) -> Option<String> {
        if let Some(source) = self.external_sources.iter().nth(0).cloned() {
            Some(source)
        } else {
            None
        }
    }

    pub fn push_source(&mut self, uri: String, buf: Vec<u8>) {
        assert!(self.external_sources.contains(&uri));

        self.external_sources.remove(&uri);
        self.buffers.insert(uri, buf);
    }

    pub fn from_file<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(path.as_ref())?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        let mut decoder = Self::new(&buf)?;

        for uri in decoder.external_sources.iter() {
            let mut path = Uri::from(path.as_ref());
            path.push(&uri);

            let mut file = File::open(path.as_path())?;

            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            decoder.buffers.insert(uri.clone(), buf);
        }

        Ok(decoder)
    }

    pub fn finish(self) -> Result<GltfData, Error> {
        let _span = trace_span!("GltfDecoder::finish").entered();

        let mut data = GltfStagingData::new(self.buffers);
        data.finish(self.gltf)?;

        Ok(GltfData {
            scenes: data.scenes,
            meshes: data.meshes,
            materials: data.materials,
            images: data.images,
            default_scene: data.default_scene,
        })
    }
}

/// An error that can occur when loading an GLTF file.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Gltf(#[from] gltf::Error),
    #[error(transparent)]
    InvalidMimeType(#[from] InvalidMimeType),
    #[error("mime-type {0:?} invalid for image")]
    NoImage(MimeType),
    #[error(transparent)]
    Base64(#[from] base64::DecodeError),
    #[error("unexpected eof")]
    Eof,
    #[error("invalid data type: {0:?}")]
    InvalidDataType(DataType),
    #[error("invalid dimensions: {0:?}")]
    InvalidDimensions(Dimensions),
    #[error("invalid buffer view {view:?} for buffer with length {length:?}")]
    InvalidBufferView { view: Range<usize>, length: usize },
    #[error("scalar value of {value} outside of valid range [{min}, {max}]")]
    ScalarOutOfRange {
        value: ScalarValue,
        min: ScalarValue,
        max: ScalarValue,
    },
    #[error("invalid acessor value: {0}")]
    InvalidAccessor(#[from] InvalidAccessorValue),
    #[error("failed to load image: {0}")]
    LoadImage(#[from] ::image::ImageError),
}

/// A parsed glTF file.
#[derive(Clone, Debug)]
pub struct GltfData {
    pub scenes: Vec<GltfScene>,
    pub meshes: HashMap<MeshIndex, GltfMesh>,
    pub materials: HashMap<MaterialIndex, GltfMaterial>,
    pub images: HashMap<TextureIndex, Image>,
    pub default_scene: Option<usize>,
}

impl GltfData {
    pub fn default_scene(&self) -> Option<&GltfScene> {
        self.default_scene
            .map(|index| self.scenes.get(index).unwrap())
    }

    pub fn from_file<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        GltfDecoder::from_file(path)?.finish()
    }
}

#[derive(Clone, Debug)]
struct GltfStagingData {
    buffers: HashMap<String, Vec<u8>>,
    meshes: HashMap<MeshIndex, GltfMesh>,
    scenes: Vec<GltfScene>,
    images: HashMap<TextureIndex, Image>,
    materials: HashMap<MaterialIndex, GltfMaterial>,
    default_scene: Option<usize>,
}

impl GltfStagingData {
    fn new(buffers: HashMap<String, Vec<u8>>) -> Self {
        Self {
            buffers,
            materials: HashMap::new(),
            scenes: vec![],
            images: HashMap::new(),
            meshes: HashMap::new(),
            default_scene: None,
        }
    }

    pub fn finish(&mut self, gltf: Gltf) -> Result<(), Error> {
        let mut scenes = Vec::new();

        if let Some(scene) = gltf.default_scene() {
            self.default_scene = Some(scene.index());
        }

        for scene in gltf.scenes() {
            let mut nodes = Hierarchy::new();

            let mut parents = BTreeMap::new();

            for node in scene.nodes() {
                let parent = nodes.append(
                    None,
                    GltfNode {
                        transform: Transform::default(),
                        mesh: None,
                        material: None,
                        name: None,
                    },
                );

                for node in self.load_node(&node)? {
                    nodes.append(Some(parent), node);
                }

                if !node.children().len() != 0 {
                    for child in node.children() {
                        parents.insert(child.index(), parent);
                    }
                }
            }

            while !parents.is_empty() {
                for (child, parent) in parents.clone().iter() {
                    let parent = nodes.append(
                        Some(*parent),
                        GltfNode {
                            transform: Transform::default(),
                            mesh: None,
                            material: None,
                            name: None,
                        },
                    );

                    let node = gltf.nodes().nth(*child).unwrap();
                    for node in self.load_node(&node)? {
                        nodes.append(Some(parent), node);
                    }

                    parents.remove(child);
                    for child in node.children() {
                        parents.insert(child.index(), parent);
                    }
                }
            }

            scenes.push(GltfScene { nodes });
        }

        self.scenes = scenes;
        Ok(())
    }

    // Note that in gltf a single node can contain multiple "primitives" which are
    // already formed like a node (with mesh + material). We flatten this hierarchy
    // into a list of nodes instead.
    fn load_node(&mut self, node: &Node<'_>) -> Result<Vec<GltfNode>, Error> {
        let meshes = if let Some(mesh) = node.mesh() {
            self.load_node_meshes(mesh)?
        } else {
            vec![]
        };

        let (translation, rotation, scale) = node.transform().decomposed();
        let transform = Transform {
            translation: Vec3::from_array(translation),
            rotation: Quat::from_array(rotation),
            scale: Vec3::from_array(scale),
        };

        // TODO: Error instead of panicking.
        assert!(transform.rotation.is_normalized());

        Ok(meshes
            .into_iter()
            .map(|primitive| GltfNode {
                transform,
                mesh: Some(primitive.mesh),
                material: Some(primitive.material),
                name: node.name().map(|s| s.to_owned()),
            })
            .collect())
    }

    fn load_node_meshes(&mut self, mesh: gltf::Mesh<'_>) -> Result<Vec<GltfMeshMaterial>, Error> {
        let mut meshes_out = Vec::new();

        for primitive in mesh.primitives() {
            let mesh = self.load_mesh(&primitive, mesh.index())?;
            let material = self.load_material(primitive.material())?;

            //mesh::validate_mesh(&mesh);

            meshes_out.push(GltfMeshMaterial { mesh, material });
        }

        Ok(meshes_out)
    }

    fn load_mesh(
        &mut self,
        primitive: &gltf::Primitive<'_>,
        mesh_index: usize,
    ) -> Result<MeshIndex, Error> {
        if self.meshes.contains_key(&MeshIndex {
            mesh: mesh_index,
            primitive: primitive.index(),
        }) {
            return Ok(MeshIndex {
                mesh: mesh_index,
                primitive: primitive.index(),
            });
        }

        let mut mesh = GltfMesh::default();

        assert_eq!(primitive.mode(), Mode::Triangles);

        let mut tangents_set = false;

        for (semantic, accessor) in primitive.attributes() {
            assert!(accessor.sparse().is_none());

            match semantic {
                Semantic::Positions => {
                    self.load_positions(&accessor, &mut mesh.positions)?;
                }
                Semantic::Normals => {
                    self.load_normals(&accessor, &mut mesh.normals)?;
                }
                Semantic::Tangents => {
                    self.load_tangents(&accessor, &mut mesh.tangents)?;
                    tangents_set = true;
                }
                Semantic::TexCoords(0) => {
                    self.load_uvs(&accessor, &mut mesh.uvs)?;
                }
                _ => {
                    tracing::warn!(
                        "invalid/unsupported gltf semantic: {}",
                        semantic.to_string()
                    );
                }
            }
        }

        if let Some(accessor) = primitive.indices() {
            self.load_indices(&accessor, &mut mesh.indices)?;
        }

        if !tangents_set {
            //mesh.compute_tangents();
            //todo!()
        }

        let index = MeshIndex {
            mesh: mesh_index,
            primitive: primitive.index(),
        };
        self.meshes.insert(index, mesh);
        Ok(index)
    }

    fn buffer(&self, source: Source, offset: usize, length: usize) -> Result<&[u8], Error> {
        let buf = match source {
            Source::Bin => {
                let buf = self.buffers.get("").unwrap();
                buf
            }
            Source::Uri(uri) => {
                let buf = self.buffers.get(uri).unwrap();
                buf
            }
        };

        match buf.get(offset..offset + length) {
            Some(buf) => Ok(buf),
            None => Err(Error::InvalidBufferView {
                view: offset..offset + length,
                length: buf.len(),
            }),
        }
    }

    fn load_positions(&self, accessor: &Accessor, positions: &mut Vec<Vec3>) -> Result<(), Error> {
        let data_type = accessor.data_type();
        if data_type != DataType::F32 {
            return Err(Error::InvalidDataType(data_type));
        }

        let dimensions = accessor.dimensions();
        if dimensions != Dimensions::Vec3 {
            return Err(Error::InvalidDimensions(dimensions));
        }

        let min = accessor
            .min()
            .map(|min| AccessorValue::load(Dimensions::Vec3, data_type, min));

        let reader: ItemReader<'_, Positions> = ItemReader::new(accessor, self);
        positions.extend(reader.map(Vec3::from_array));

        Ok(())
    }

    fn load_normals(&self, accessor: &Accessor, normals: &mut Vec<Vec3>) -> Result<(), Error> {
        let data_type = accessor.data_type();
        if data_type != DataType::F32 {
            return Err(Error::InvalidDataType(data_type));
        }

        let dimensions = accessor.dimensions();
        if dimensions != Dimensions::Vec3 {
            return Err(Error::InvalidDimensions(dimensions));
        }

        let reader: ItemReader<'_, Normals> = ItemReader::new(accessor, self);
        normals.extend(reader.map(Vec3::from_array));

        Ok(())
    }

    fn load_tangents(&self, accessor: &Accessor, tangents: &mut Vec<Vec4>) -> Result<(), Error> {
        let data_type = accessor.data_type();
        if data_type != DataType::F32 {
            return Err(Error::InvalidDataType(data_type));
        }

        let dimensions = accessor.dimensions();
        if dimensions != Dimensions::Vec4 {
            return Err(Error::InvalidDimensions(dimensions));
        }

        let reader: ItemReader<'_, Tangents> = ItemReader::new(accessor, self);
        tangents.extend(reader.map(|arr| Vec4::from_array(arr)));

        Ok(())
    }

    fn load_uvs(&self, accessor: &Accessor, uvs: &mut Vec<Vec2>) -> Result<(), Error> {
        let data_type = accessor.data_type();
        if data_type != DataType::F32 {
            return Err(Error::InvalidDataType(data_type));
        }

        let dimensions = accessor.dimensions();
        if dimensions != Dimensions::Vec2 {
            return Err(Error::InvalidDimensions(dimensions));
        }

        let reader: ItemReader<'_, Uvs> = ItemReader::new(accessor, self);
        uvs.extend(reader.map(Vec2::from_array));

        Ok(())
    }

    fn load_indices(&self, accessor: &Accessor, indices: &mut Vec<u32>) -> Result<(), Error> {
        let data_type = accessor.data_type();

        let dimensions = accessor.dimensions();
        if dimensions != Dimensions::Scalar {
            return Err(Error::InvalidDimensions(dimensions));
        }

        let alignment = match data_type {
            DataType::U16 => 2,
            DataType::U32 => 4,
            _ => return Err(Error::InvalidDataType(data_type)),
        };

        let view = accessor.view().unwrap();

        // viewOffset MUST be a multiple of the component type, i.e. correctly
        // aligned.
        assert!(view.offset() % alignment == 0);

        match data_type {
            DataType::U16 => {
                let reader: ItemReader<'_, u16> = ItemReader::new(accessor, self);
                indices.extend(reader.map(u32::from));
            }
            DataType::U32 => {
                let reader: ItemReader<'_, u32> = ItemReader::new(accessor, self);
                indices.extend(reader);
            }
            _ => (),
        }

        assert!(
            indices.len() % 3 == 0,
            "Indices % 3 != 0; len = {}",
            indices.len()
        );

        Ok(())
    }

    fn load_material(&mut self, material: Material<'_>) -> Result<MaterialIndex, Error> {
        if let Some(index) = material.index() {
            if self.materials.contains_key(&MaterialIndex(index)) {
                return Ok(MaterialIndex(index));
            }
        } else {
            // `usize::MAX` should be big enough to never cause it collide with
            // a valid material index.
            const DEFAULT_MATERIAL_INDEX: usize = usize::MAX;
            if self
                .materials
                .contains_key(&MaterialIndex(DEFAULT_MATERIAL_INDEX))
            {
                return Ok(MaterialIndex(DEFAULT_MATERIAL_INDEX));
            }

            // The material is undefined. We must use the default material specified
            // by the glTF spec: https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#default-material
            let material = default_material();

            self.materials
                .insert(MaterialIndex(DEFAULT_MATERIAL_INDEX), material);
            return Ok(MaterialIndex(DEFAULT_MATERIAL_INDEX));
        }

        let alpha_mode = material.alpha_mode();

        let pbr = material.pbr_metallic_roughness();

        let base_color = pbr.base_color_factor();

        let base_color_texture = if let Some(info) = pbr.base_color_texture() {
            let image = info.texture().source();

            Some(self.load_image(image, TextureFormat::Rgba8UnormSrgb)?)
        } else {
            None
        };

        let normal_texture = if let Some(info) = material.normal_texture() {
            let image = info.texture().source();

            Some(self.load_image(image, TextureFormat::Rgba8Unorm)?)
        } else {
            None
        };

        let roughness = pbr.roughness_factor();
        let metallic = pbr.metallic_factor();

        let metallic_roughness_texture = if let Some(info) = pbr.metallic_roughness_texture() {
            let image = info.texture().source();

            Some(self.load_image(image, TextureFormat::Rgba8UnormSrgb)?)
        } else {
            None
        };

        let index = material.index().unwrap();

        self.materials.insert(
            MaterialIndex(index),
            GltfMaterial {
                alpha_mode,
                base_color: Color(base_color),
                base_color_texture,
                normal_texture,
                roughness,
                metallic,
                metallic_roughness_texture,
            },
        );

        Ok(MaterialIndex(index))
    }

    fn load_image(
        &mut self,
        image: gltf::Image<'_>,
        format: TextureFormat,
    ) -> Result<TextureIndex, Error> {
        if self.images.contains_key(&TextureIndex(image.index())) {
            return Ok(TextureIndex(image.index()));
        }

        let buf = match image.source() {
            ImageSource::View { view, mime_type: _ } => {
                self.buffer(view.buffer().source(), view.offset(), view.length())
            }
            ImageSource::Uri { uri, mime_type: _ } => {
                let len = self.buffers.get(uri).unwrap().len();
                self.buffer(Source::Uri(uri), 0, len)
            }
        }?;

        let index = TextureIndex(image.index());
        let img = image::load_from_memory(buf)?.into_rgba8();
        self.images.insert(
            index,
            Image::new(
                UVec2::new(img.width(), img.height()),
                format,
                img.into_raw(),
            ),
        );
        Ok(index)
    }
}

fn read_f32(buf: &mut &[u8]) -> Result<f32, Error> {
    if buf.len() < std::mem::size_of::<f32>() {
        Err(Error::Eof)
    } else {
        Ok(buf.get_f32_le())
    }
}

fn read_u16(buf: &mut &[u8]) -> Result<u16, Error> {
    if buf.len() < std::mem::size_of::<u16>() {
        Err(Error::Eof)
    } else {
        Ok(buf.get_u16_le())
    }
}

fn read_u32(buf: &mut &[u8]) -> Result<u32, Error> {
    if buf.len() < std::mem::size_of::<u32>() {
        Err(Error::Eof)
    } else {
        Ok(buf.get_u32_le())
    }
}

fn validate_accessor_range<T>(value: T, min: T, max: T) -> Result<(), Error>
where
    T: Into<AccessorValue>,
{
    let value = value.into();
    let min = min.into();
    let max = max.into();

    match (value, min, max) {
        (AccessorValue::Scalar(value), AccessorValue::Scalar(min), AccessorValue::Scalar(max)) => {
            if value < min || value > max {
                return Err(Error::ScalarOutOfRange { value, min, max });
            }
        }
        (AccessorValue::Vec2(value), AccessorValue::Vec2(min), AccessorValue::Vec2(max)) => {
            for index in 0..2 {
                let value = value[index];
                let min = min[index];
                let max = max[index];

                if value < min || value > max {
                    return Err(Error::ScalarOutOfRange { value, min, max });
                }
            }
        }
        (AccessorValue::Vec3(value), AccessorValue::Vec3(min), AccessorValue::Vec3(max)) => {
            for index in 0..3 {
                let value = value[index];
                let min = min[index];
                let max = max[index];

                if value < min || value > max {
                    return Err(Error::ScalarOutOfRange { value, min, max });
                }
            }
        }
        (AccessorValue::Vec4(value), AccessorValue::Vec4(min), AccessorValue::Vec4(max)) => {
            for index in 0..4 {
                let value = value[index];
                let min = min[index];
                let max = max[index];

                if value < min || value > max {
                    return Err(Error::ScalarOutOfRange { value, min, max });
                }
            }
        }
        _ => todo!(),
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq, Error)]
pub enum InvalidAccessorValue {
    #[error("invalid dimensions: {0}, expected {1:?}")]
    InvalidDimensions(u64, Dimensions),
    #[error("invalid scalar: {0}")]
    InvalidScalar(#[from] InvalidScalar),
    #[error("no value: {0}")]
    NoArray(serde_json::Value),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AccessorValue {
    Scalar(ScalarValue),
    Vec2([ScalarValue; 2]),
    Vec3([ScalarValue; 3]),
    Vec4([ScalarValue; 4]),
    Mat2([[ScalarValue; 2]; 2]),
    Mat3([[ScalarValue; 3]; 3]),
    Mat4([[ScalarValue; 4]; 4]),
}

impl AccessorValue {
    fn load(
        dimensions: Dimensions,
        data_type: DataType,
        value: serde_json::Value,
    ) -> Result<Self, InvalidAccessorValue> {
        match dimensions {
            Dimensions::Scalar => {
                let e = ScalarValue::load(data_type, value)?;
                Ok(Self::Scalar(e))
            }
            Dimensions::Vec2 => match value.as_array() {
                Some(array) => {
                    if array.len() != 2 {
                        return Err(InvalidAccessorValue::InvalidDimensions(
                            array.len() as u64,
                            Dimensions::Vec2,
                        ));
                    }

                    let e0 = ScalarValue::load(data_type, array[0].clone())?;
                    let e1 = ScalarValue::load(data_type, array[1].clone())?;

                    Ok(Self::Vec2([e0, e1]))
                }
                None => Err(InvalidAccessorValue::NoArray(value)),
            },
            Dimensions::Vec3 => match value.as_array() {
                Some(array) => {
                    if array.len() != 3 {
                        return Err(InvalidAccessorValue::InvalidDimensions(
                            array.len() as u64,
                            Dimensions::Vec3,
                        ));
                    }

                    let e0 = ScalarValue::load(data_type, array[0].clone())?;
                    let e1 = ScalarValue::load(data_type, array[1].clone())?;
                    let e2 = ScalarValue::load(data_type, array[2].clone())?;

                    Ok(Self::Vec3([e0, e1, e2]))
                }
                None => Err(InvalidAccessorValue::NoArray(value)),
            },
            Dimensions::Vec4 => match value.as_array() {
                Some(array) => {
                    if array.len() != 4 {
                        return Err(InvalidAccessorValue::InvalidDimensions(
                            array.len() as u64,
                            Dimensions::Vec4,
                        ));
                    }

                    let e0 = ScalarValue::load(data_type, array[0].clone())?;
                    let e1 = ScalarValue::load(data_type, array[1].clone())?;
                    let e2 = ScalarValue::load(data_type, array[2].clone())?;
                    let e3 = ScalarValue::load(data_type, array[3].clone())?;

                    Ok(Self::Vec4([e0, e1, e2, e3]))
                }
                None => Err(InvalidAccessorValue::NoArray(value)),
            },
            _ => todo!(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum ScalarValue {
    U8(u8),
    U16(u16),
    U32(u32),
    I8(i8),
    I16(i16),
    F32(f32),
}

impl Display for ScalarValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::U8(val) => Display::fmt(val, f),
            Self::U16(val) => Display::fmt(val, f),
            Self::U32(val) => Display::fmt(val, f),
            Self::I8(val) => Display::fmt(val, f),
            Self::I16(val) => Display::fmt(val, f),
            Self::F32(val) => Display::fmt(val, f),
        }
    }
}

impl From<u8> for ScalarValue {
    #[inline]
    fn from(value: u8) -> Self {
        Self::U8(value)
    }
}

impl From<u16> for ScalarValue {
    #[inline]
    fn from(value: u16) -> Self {
        Self::U16(value)
    }
}

impl From<u32> for ScalarValue {
    #[inline]
    fn from(value: u32) -> Self {
        Self::U32(value)
    }
}

impl From<i8> for ScalarValue {
    #[inline]
    fn from(value: i8) -> Self {
        Self::I8(value)
    }
}

impl From<i16> for ScalarValue {
    #[inline]
    fn from(value: i16) -> Self {
        Self::I16(value)
    }
}

impl From<f32> for ScalarValue {
    #[inline]
    fn from(value: f32) -> Self {
        Self::F32(value)
    }
}

impl ScalarValue {
    fn load(data_type: DataType, value: serde_json::Value) -> Result<Self, InvalidScalar> {
        match value {
            Value::Number(number) => match data_type {
                DataType::U8 => match number.as_u64().map(|val| val.try_into().ok()).flatten() {
                    Some(val) => Ok(Self::U8(val)),
                    None => Err(InvalidScalar::InvalidU8(number)),
                },
                DataType::U16 => match number.as_u64().map(|val| val.try_into().ok()).flatten() {
                    Some(val) => Ok(Self::U16(val)),
                    None => Err(InvalidScalar::InvalidU16(number)),
                },
                DataType::U32 => match number.as_u64().map(|val| val.try_into().ok()).flatten() {
                    Some(val) => Ok(Self::U32(val)),
                    None => Err(InvalidScalar::InvalidU32(number)),
                },
                DataType::I8 => match number.as_i64().map(|val| val.try_into().ok()).flatten() {
                    Some(val) => Ok(Self::I8(val)),
                    None => Err(InvalidScalar::InvalidI8(number)),
                },
                DataType::I16 => match number.as_i64().map(|val| val.try_into().ok()).flatten() {
                    Some(val) => Ok(Self::I16(val)),
                    None => Err(InvalidScalar::InvalidI16(number)),
                },
                DataType::F32 => match number.as_f64() {
                    Some(val) => Ok(Self::F32(val as f32)),
                    None => Err(InvalidScalar::InvalidF32(number)),
                },
            },
            _ => Err(InvalidScalar::NoScalar(value)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Error)]
pub enum InvalidScalar {
    #[error("not a scalar value: {0}")]
    NoScalar(Value),
    #[error("invalid u8: {0}")]
    InvalidU8(Number),
    #[error("invalid u16: {0}")]
    InvalidU16(Number),
    #[error("invalid u32: {0}")]
    InvalidU32(Number),
    #[error("invalid i8: {0}")]
    InvalidI8(Number),
    #[error("invalid i16: {0}")]
    InvalidI16(Number),
    #[error("invalid f32: {0}")]
    InvalidF32(Number),
}

/// Returns the default material.
fn default_material() -> GltfMaterial {
    // The default material values as specified by
    // https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-material

    GltfMaterial {
        alpha_mode: AlphaMode::Opaque,
        base_color: Color([1.0, 1.0, 1.0, 1.0]),
        base_color_texture: None,
        metallic: 1.0,
        roughness: 1.0,
        metallic_roughness_texture: None,
        normal_texture: None,
    }
}
