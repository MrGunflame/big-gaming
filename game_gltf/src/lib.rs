mod image;
mod material;
mod mesh;
mod mime;
mod scene;

pub mod uri;

use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::ops::Range;
use std::path::Path;

use base64::alphabet::STANDARD;
use base64::engine::GeneralPurpose;
use base64::engine::GeneralPurposeConfig;
use base64::Engine;
use bytes::Buf;
use game_common::components::transform::Transform;
use game_render::color::Color;
use game_render::mesh::Indices;
use game_render::mesh::Mesh;
use game_render::pbr::AlphaMode;
use glam::{Quat, Vec3};
use gltf::accessor::DataType;
use gltf::accessor::Dimensions;
use gltf::buffer::Source;
use gltf::mesh::Mode;
use gltf::Material;
use gltf::{Accessor, Gltf, Semantic};
use gltf::{Image, Node};
use indexmap::IndexMap;
use indexmap::IndexSet;
use mime::InvalidMimeType;
use mime::MimeType;
use serde_json::{Number, Value};
use thiserror::Error;
use uri::Uri;

pub use material::GltfMaterial;
pub use mesh::{GltfMesh, GltfNode, GltfPrimitive};
pub use scene::GltfScene;

use gltf::image::Source as ImageSource;

/// A fully loaded GLTF file with buffers.
#[derive(Clone, Debug)]
pub struct GltfData {
    pub gltf: Gltf,
    pub buffers: IndexMap<String, Vec<u8>>,
}

/// A GLTF file that is being loaded.
///
/// A (non-binary) GLTF file may reference to buffers from external URIs that have to be loaded
/// before the data in the GLTF file can be accessed.
///
/// The URIs that are required for this GLTF file are stored in `queue`.
#[derive(Clone, Debug)]
pub struct GltfLoader {
    data: GltfData,
    // FIXME: This could be &str since the string buffer
    // is already in self.gltf.
    pub queue: IndexSet<String>,
}

impl GltfLoader {
    pub fn insert(&mut self, uri: String, buf: Vec<u8>) {
        self.queue.remove(&uri);
        self.data.buffers.insert(uri.to_owned(), buf.to_vec());
    }

    pub fn create(self) -> GltfData {
        assert!(self.queue.is_empty());
        self.create_unchecked()
    }

    pub fn create_unchecked(self) -> GltfData {
        self.data
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

impl GltfData {
    pub fn new(slice: &[u8]) -> Result<GltfLoader, Error> {
        let gltf = Gltf::from_slice(slice)?;
        let mut queue = IndexSet::new();
        let mut buffers = IndexMap::new();

        for buffer in gltf.buffers() {
            match buffer.source() {
                Source::Bin => {
                    buffers.insert(String::from(""), gltf.blob.clone().unwrap());
                }
                Source::Uri(uri) => {
                    if let Some(data) = uri.strip_prefix("data:application/octet-stream;base64,") {
                        let engine = GeneralPurpose::new(&STANDARD, GeneralPurposeConfig::new());
                        let buf = engine.decode(data)?;

                        buffers.insert(uri.to_owned(), buf);
                    } else {
                        queue.insert(uri.to_owned());
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

                queue.insert(uri.to_owned());
            }
        }

        Ok(GltfLoader {
            data: GltfData { gltf, buffers },
            queue,
        })
    }

    pub fn open<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(path.as_ref())?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        let mut loader = Self::new(&buf)?;

        while let Some(uri) = loader.queue.swap_remove_index(0) {
            let mut path = Uri::from(path.as_ref());
            path.push(&uri);

            let mut file = File::open(path.as_path())?;

            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            loader.insert(uri, buf);
        }

        Ok(loader.create_unchecked())
    }

    // FIXME: Do we want to have validation on accessor methods, or do it
    // on loading the object instead?.
    pub fn scenes(&self) -> Result<Vec<GltfScene>, Error> {
        let mut scenes = Vec::new();

        for scene in self.gltf.scenes() {
            let mut nodes = Vec::new();
            for node in scene.nodes() {
                nodes.push(self.load_node(node)?);
            }

            scenes.push(GltfScene { nodes });
        }

        Ok(scenes)
    }

    fn load_node(&self, node: Node<'_>) -> Result<GltfNode, Error> {
        let children = node
            .children()
            .into_iter()
            .map(|node| self.load_node(node))
            .collect::<Result<Vec<_>, Error>>()?;

        let mesh = if let Some(mesh) = node.mesh() {
            Some(self.load_mesh(mesh)?)
        } else {
            None
        };

        let (translation, rotation, scale) = node.transform().decomposed();
        let transform = Transform {
            translation: Vec3::from_array(translation),
            rotation: Quat::from_array(rotation),
            scale: Vec3::from_array(scale),
        };

        // TODO: Error instead of panicking.
        assert!(transform.rotation.is_normalized());

        Ok(GltfNode {
            children,
            mesh,
            transform,
        })
    }

    fn load_mesh(&self, mesh: gltf::Mesh<'_>) -> Result<GltfMesh, Error> {
        let mut primitives = Vec::new();

        for primitive in mesh.primitives() {
            let mut mesh = Mesh::new();

            assert_eq!(primitive.mode(), Mode::Triangles);

            for (semantic, accessor) in primitive.attributes() {
                match semantic {
                    Semantic::Positions => {
                        let mut positions = Vec::new();
                        self.load_positions(&accessor, &mut positions)?;
                        mesh.set_positions(positions);
                    }
                    Semantic::Normals => {
                        let mut normals = Vec::new();
                        self.load_normals(&accessor, &mut normals)?;
                        mesh.set_normals(normals);
                    }
                    Semantic::TexCoords(index) => {
                        if index != 0 {
                            panic!("multiple texture coordinates not yet supported");
                        }

                        let mut uvs = vec![];
                        self.load_uvs(&accessor, &mut uvs)?;
                        mesh.set_uvs(uvs);
                    }
                    Semantic::Tangents => {
                        let mut tangents = vec![];
                        self.load_tangents(&accessor, &mut tangents)?;
                        let t = tangents;
                    }
                    _ => {
                        todo!()
                    }
                }
            }

            if let Some(accessor) = primitive.indices() {
                let mut indices = Indices::U16(vec![]);
                self.load_indices(&accessor, &mut indices)?;
                mesh.set_indices(indices);
            }

            let material = self.load_material(primitive.material())?;

            primitives.push(GltfPrimitive { mesh, material });
        }

        Ok(GltfMesh { primitives })
    }

    fn buffer(&self, source: Source, offset: usize, length: usize) -> Result<&[u8], Error> {
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

        match buf.get(offset..offset + length) {
            Some(buf) => Ok(buf),
            None => Err(Error::InvalidBufferView {
                view: offset..offset + length,
                length: buf.len(),
            }),
        }
    }

    fn load_positions(
        &self,
        accessor: &Accessor,
        positions: &mut Vec<[f32; 3]>,
    ) -> Result<(), Error> {
        let data_type = accessor.data_type();
        if data_type != DataType::F32 {
            return Err(Error::InvalidDataType(data_type));
        }

        let dimensions = accessor.dimensions();
        if dimensions != Dimensions::Vec3 {
            return Err(Error::InvalidDimensions(dimensions));
        }

        let view = accessor.view().unwrap();
        let buffer = view.buffer();

        let mut buf = self.buffer(buffer.source(), view.offset(), view.length())?;

        match (accessor.min(), accessor.max()) {
            (Some(min), Some(max)) => {
                let min = AccessorValue::load(Dimensions::Vec3, data_type, min)?;
                let max = AccessorValue::load(Dimensions::Vec3, data_type, max)?;

                while buf.len() != 0 {
                    let x = read_f32(&mut buf)?;
                    let y = read_f32(&mut buf)?;
                    let z = read_f32(&mut buf)?;

                    validate_accessor_range(
                        AccessorValue::Vec3([x.into(), y.into(), z.into()]),
                        min,
                        max,
                    )?;

                    positions.push([x, y, z]);
                }
            }
            _ => {
                while buf.len() != 0 {
                    let x = read_f32(&mut buf)?;
                    let y = read_f32(&mut buf)?;
                    let z = read_f32(&mut buf)?;

                    positions.push([x, y, z]);
                }
            }
        }

        Ok(())
    }

    fn load_normals(&self, accessor: &Accessor, normals: &mut Vec<[f32; 3]>) -> Result<(), Error> {
        let data_type = accessor.data_type();
        if data_type != DataType::F32 {
            return Err(Error::InvalidDataType(data_type));
        }

        let dimensions = accessor.dimensions();
        if dimensions != Dimensions::Vec3 {
            return Err(Error::InvalidDimensions(dimensions));
        }

        let view = accessor.view().unwrap();
        let buffer = view.buffer();

        let mut buf = self.buffer(buffer.source(), view.offset(), view.length())?;

        while buf.len() != 0 {
            let x = read_f32(&mut buf)?;
            let y = read_f32(&mut buf)?;
            let z = read_f32(&mut buf)?;

            normals.push([x, y, z]);
        }

        Ok(())
    }

    fn load_tangents(
        &self,
        accessor: &Accessor,
        tangents: &mut Vec<[f32; 4]>,
    ) -> Result<(), Error> {
        let data_type = accessor.data_type();
        if data_type != DataType::F32 {
            return Err(Error::InvalidDataType(data_type));
        }

        let dimensions = accessor.dimensions();
        if dimensions != Dimensions::Vec4 {
            return Err(Error::InvalidDimensions(dimensions));
        }

        let view = accessor.view().unwrap();
        let buffer = view.buffer();

        let mut buf = self.buffer(buffer.source(), view.offset(), view.length())?;

        while buf.len() != 0 {
            let x = read_f32(&mut buf)?;
            let y = read_f32(&mut buf)?;
            let z = read_f32(&mut buf)?;
            let w = read_f32(&mut buf)?;

            tangents.push([x, y, z, w]);
        }

        Ok(())
    }

    fn load_uvs(&self, accessor: &Accessor, uvs: &mut Vec<[f32; 2]>) -> Result<(), Error> {
        let data_type = accessor.data_type();
        if data_type != DataType::F32 {
            return Err(Error::InvalidDataType(data_type));
        }

        let dimensions = accessor.dimensions();
        if dimensions != Dimensions::Vec2 {
            return Err(Error::InvalidDimensions(dimensions));
        }

        let view = accessor.view().unwrap();
        let buffer = view.buffer();

        let mut buf = self.buffer(buffer.source(), view.offset(), view.length())?;

        while buf.len() != 0 {
            let x = read_f32(&mut buf)?;
            let y = read_f32(&mut buf)?;

            uvs.push([x, y]);
        }

        Ok(())
    }

    fn load_indices(&self, accessor: &Accessor, indices: &mut Indices) -> Result<(), Error> {
        let data_type = accessor.data_type();

        let dimensions = accessor.dimensions();
        if dimensions != Dimensions::Scalar {
            return Err(Error::InvalidDimensions(dimensions));
        }

        let view = accessor.view().unwrap();
        let buffer = view.buffer();

        let mut buf = self.buffer(buffer.source(), view.offset(), view.length())?;

        match data_type {
            DataType::U16 => {
                let mut out = vec![];

                while buf.len() != 0 {
                    let val = read_u16(&mut buf)?;
                    out.push(val);
                }

                *indices = Indices::U16(out);
            }
            DataType::U32 => {
                let mut out = vec![];

                while buf.len() != 0 {
                    let val = read_u32(&mut buf)?;
                    out.push(val);
                }

                *indices = Indices::U32(out);
            }
            _ => return Err(Error::InvalidDataType(data_type)),
        }

        Ok(())
    }

    fn load_material(&self, material: Material<'_>) -> Result<GltfMaterial, Error> {
        let alpha_mode = convert_alpha_mode(material.alpha_mode());

        let pbr = material.pbr_metallic_roughness();

        let base_color = pbr.base_color_factor();

        let base_color_texture = if let Some(info) = pbr.base_color_texture() {
            let image = info.texture().source();

            let buf = self.load_image(image)?;
            Some(buf.to_vec())
        } else {
            None
        };

        let normal_texture = if let Some(info) = material.normal_texture() {
            let image = info.texture().source();

            let buf = self.load_image(image)?;
            Some(buf.to_vec())
        } else {
            None
        };

        let roughness = pbr.roughness_factor();
        let metallic = pbr.metallic_factor();

        let metallic_roughness_texture = if let Some(info) = pbr.metallic_roughness_texture() {
            let image = info.texture().source();

            let buf = self.load_image(image)?;
            Some(buf.to_vec())
        } else {
            None
        };

        Ok(GltfMaterial {
            alpha_mode,
            base_color: Color(base_color),
            base_color_texture,
            normal_texture,
            roughness,
            metallic,
            metallic_roughness_texture,
        })
    }

    fn load_image(&self, image: Image<'_>) -> Result<&[u8], Error> {
        match image.source() {
            ImageSource::View { view, mime_type: _ } => {
                self.buffer(view.buffer().source(), view.offset(), view.length())
            }
            ImageSource::Uri { uri, mime_type: _ } => {
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
