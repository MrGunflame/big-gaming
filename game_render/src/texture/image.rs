use std::path::PathBuf;

use bytes::Bytes;
use glam::UVec2;

use crate::backend::TextureFormat;

#[derive(Clone, Debug)]
pub struct MipImage {
    mips: Vec<Image>,
}

impl MipImage {
    pub fn new(mips: Vec<Image>) -> Self {
        assert!(!mips.is_empty());

        Self { mips }
    }

    pub fn format(&self) -> TextureFormat {
        self.mips[0].format()
    }

    pub fn mip_levels(&self) -> u32 {
        self.mips.len() as u32
    }

    pub fn root(&self) -> &Image {
        &self.mips[0]
    }

    pub fn get(&self, level: u32) -> Option<&Image> {
        self.mips.get(level as usize)
    }

    pub fn mips(&self) -> &[Image] {
        &self.mips
    }
}

impl From<Image> for MipImage {
    fn from(value: Image) -> Self {
        Self::new(vec![value])
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageFormat {
    Png,
}

#[derive(Clone, Debug)]
pub struct Image {
    bytes: Bytes,
    format: TextureFormat,
    width: u32,
    height: u32,
}

impl Image {
    pub fn new<T>(size: UVec2, format: TextureFormat, bytes: T) -> Self
    where
        T: Into<Bytes>,
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
        let expected = self
            .format
            .storage_size(UVec2::new(self.width, self.height));
        assert_eq!(
            self.bytes.len(),
            expected,
            "expected {} bytes, got {} bytes for image with dimensions ({}, {}) and format {:?}",
            expected,
            self.bytes.len(),
            self.width,
            self.height,
            self.format,
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

        if self.bytes.len() != other.bytes.len() {
            return false;
        }

        // The images always the same if they refer to
        // the same allocation. This check avoids us avoid
        // a big memcmp if true.
        if self.bytes.as_ptr() == other.bytes.as_ptr() {
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
