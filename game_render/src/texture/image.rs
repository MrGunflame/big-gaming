use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use glam::UVec2;
use parking_lot::Mutex;

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

#[derive(Debug, Default)]
pub struct Images {
    next_id: u64,
    images: HashMap<u64, Entry>,
    load_queue: VecDeque<(u64, LoadImage, TextureFormat)>,
    events: Arc<Mutex<VecDeque<Event>>>,
}

impl Images {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            images: HashMap::new(),
            load_queue: VecDeque::new(),
            events: Arc::default(),
        }
    }

    pub fn insert(&mut self, image: Image) -> ImageHandle {
        let id = self.next_id();
        self.images.insert(
            id,
            Entry {
                data: image,
                ref_count: 1,
            },
        );
        ImageHandle {
            id,
            events: self.events.clone(),
        }
    }

    pub fn get(&self, handle: &ImageHandle) -> Option<&Image> {
        self.images.get(&handle.id).map(|entry| &entry.data)
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn load<S>(&mut self, source: S) -> ImageHandle
    where
        S: Into<LoadImage>,
    {
        self.load_with_format(source, TextureFormat::Rgba8UnormSrgb)
    }

    pub fn load_with_format<S>(&mut self, source: S, format: TextureFormat) -> ImageHandle
    where
        S: Into<LoadImage>,
    {
        let id = self.next_id();
        self.load_queue.push_back((id, source.into(), format));
        ImageHandle {
            id,
            events: self.events.clone(),
        }
    }
}

#[derive(Debug)]
struct Entry {
    data: Image,
    ref_count: usize,
}

#[derive(Debug)]
pub struct ImageHandle {
    id: u64,
    events: Arc<Mutex<VecDeque<Event>>>,
}

impl Clone for ImageHandle {
    fn clone(&self) -> Self {
        self.events.lock().push_back(Event::Clone(self.id));

        Self {
            id: self.id,
            events: self.events.clone(),
        }
    }
}

impl Drop for ImageHandle {
    fn drop(&mut self) {
        self.events.lock().push_back(Event::Drop(self.id));
    }
}

pub(crate) fn load_images(images: &mut Images) {
    while let Some((handle, source, format)) = images.load_queue.pop_front() {
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

        images.images.insert(
            handle,
            Entry {
                data: Image {
                    format,
                    width: img.width(),
                    height: img.height(),
                    bytes: img.into_raw(),
                },
                ref_count: 1,
            },
        );
    }
}

pub(crate) fn update_image_handles(images: &mut Images) {
    let mut events = images.events.lock();
    while let Some(event) = events.pop_front() {
        match event {
            Event::Clone(id) => {
                let entry = images.images.get_mut(&id).unwrap();
                entry.ref_count += 1;
            }
            Event::Drop(id) => {
                let entry = images.images.get_mut(&id).unwrap();
                entry.ref_count -= 1;

                if entry.ref_count == 0 {
                    images.images.remove(&id);
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Event {
    Drop(u64),
    Clone(u64),
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
