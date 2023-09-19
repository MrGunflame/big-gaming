use std::collections::VecDeque;
use std::io::Read;
use std::path::PathBuf;

use glam::UVec2;

use slotmap::{DefaultKey, SlotMap};
pub use wgpu::TextureFormat;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageFormat {
    Png,
}

#[derive(Clone, Debug)]
pub struct Image {
    bytes: Vec<u8>,
    format: TextureFormat,
    width: u32,
    height: u32,
}

impl Image {
    pub fn new(size: UVec2, format: TextureFormat, bytes: Vec<u8>) -> Self {
        let this = Self {
            bytes,
            format,
            width: size.x,
            height: size.y,
        };

        this.validate_size();
        this
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    fn validate_size(&self) {
        use TextureFormat::*;

        let size = match self.format {
            // 8-bit
            R8Unorm => 1,
            R8Snorm => 1,
            R8Uint => 1,
            R8Sint => 1,
            // 16-bit
            R16Uint => 2,
            R16Sint => 2,
            R16Float => 2,
            Rg8Unorm => 2,
            Rg8Snorm => 2,
            Rg8Uint => 2,
            Rg8Sint => 2,
            // 32-bit
            R32Uint => 4,
            R32Sint => 4,
            R32Float => 4,
            Rg16Uint => 4,
            Rg16Sint => 4,
            Rg16Float => 4,
            Rgba8Unorm => 4,
            Rgba8UnormSrgb => 4,
            Rgba8Snorm => 4,
            Rgba8Uint => 4,
            Rgba8Sint => 4,
            Bgra8Unorm => 4,
            Bgra8UnormSrgb => 4,
            _ => panic!("unsupported texture format: {:?}", self.format),
        };

        assert_eq!(
            self.width as usize * self.height as usize * size as usize,
            self.bytes.len()
        );
    }
}

impl AsRef<[u8]> for Image {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImageId(DefaultKey);

#[derive(Clone, Debug)]
enum Entry {
    Image(Image),
    Loading,
}

#[derive(Debug, Default)]
pub struct Images {
    images: SlotMap<DefaultKey, Entry>,
    load_queue: VecDeque<(DefaultKey, LoadImage, TextureFormat)>,
}

impl Images {
    pub fn new() -> Self {
        Self {
            images: SlotMap::new(),
            load_queue: VecDeque::new(),
        }
    }

    pub fn insert(&mut self, image: Image) -> ImageId {
        let id = self.images.insert(Entry::Image(image));
        ImageId(id)
    }

    pub fn get(&self, id: ImageId) -> Option<&Image> {
        match self.images.get(id.0)? {
            Entry::Image(img) => Some(img),
            Entry::Loading => None,
        }
    }

    pub fn load<S>(&mut self, source: S) -> ImageId
    where
        S: Into<LoadImage>,
    {
        self.load_with_format(source, TextureFormat::Rgba8UnormSrgb)
    }

    pub fn load_with_format<S>(&mut self, source: S, format: TextureFormat) -> ImageId
    where
        S: Into<LoadImage>,
    {
        let key = self.images.insert(Entry::Loading);

        self.load_queue.push_back((key, source.into(), format));
        ImageId(key)
    }
}

pub(crate) fn load_images(images: &mut Images) {
    while let Some((key, source, format)) = images.load_queue.pop_front() {
        let buf = match source {
            LoadImage::Buffer(buf) => buf,
            LoadImage::File(path) => {
                let mut file = std::fs::File::open(path).unwrap();

                let mut buf = Vec::new();
                file.read_to_end(&mut buf).unwrap();
                buf
            }
        };

        let img = image::load_from_memory(&buf).unwrap().to_rgba8();

        if let Some(entry) = images.images.get_mut(key) {
            *entry = Entry::Image(Image::new(
                UVec2 {
                    x: img.width(),
                    y: img.height(),
                },
                format,
                img.into_raw(),
            ));
        }
    }
}

#[derive(Clone, Debug)]
pub enum LoadImage {
    Buffer(Vec<u8>),
    File(PathBuf),
}

impl From<Vec<u8>> for LoadImage {
    fn from(value: Vec<u8>) -> Self {
        Self::Buffer(value)
    }
}

impl From<PathBuf> for LoadImage {
    fn from(value: PathBuf) -> Self {
        Self::File(value)
    }
}

impl<'a> From<&'a str> for LoadImage {
    fn from(value: &'a str) -> Self {
        Self::from(PathBuf::from(value))
    }
}
