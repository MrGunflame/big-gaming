use std::path::PathBuf;
use std::sync::Arc;

use glam::UVec2;

pub use wgpu::TextureFormat;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageFormat {
    Png,
}

#[derive(Clone, Debug)]
pub struct Image {
    bytes: Arc<[u8]>,
    format: TextureFormat,
    width: u32,
    height: u32,
}

impl Image {
    pub fn new<T>(size: UVec2, format: TextureFormat, bytes: T) -> Self
    where
        T: Into<Arc<[u8]>>,
    {
        let this = Self {
            bytes: bytes.into(),
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

impl PartialEq for &Image {
    fn eq(&self, other: &Self) -> bool {
        if self.format != other.format || self.width != other.width || self.height != other.height {
            return false;
        }

        // The images always the same if they refer to
        // the same allocation. This check avoids us avoid
        // a big memcmp if true.
        if Arc::ptr_eq(&self.bytes, &other.bytes) {
            return true;
        }

        self.bytes == other.bytes
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
