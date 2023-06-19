use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use bevy_app::{App, Plugin};
use bevy_ecs::system::{ResMut, Resource};
use game_asset::{Asset, LoadAsset};
use image::load_from_memory;
use parking_lot::Mutex;

pub use wgpu::TextureFormat;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageFormat {
    Png,
}

#[derive(Clone, Debug)]
pub struct Image {
    pub bytes: Vec<u8>,
    pub format: TextureFormat,
    pub width: u32,
    pub height: u32,
}

impl Asset for Image {}

impl LoadAsset for Image {
    type Error = Box<dyn std::error::Error>;

    fn load(bytes: &[u8]) -> Result<Self, Self::Error> {
        let img = load_from_memory(bytes)?;
        let data = img.to_rgb8();

        Ok(Self {
            width: data.width(),
            height: data.height(),
            bytes: data.into_raw(),
            format: TextureFormat::Rgba8Unorm,
        })
    }
}

pub struct ImagePlugin;

impl Plugin for ImagePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Images::new());

        app.add_system(load_images);
        app.add_system(update_image_handles);
    }
}

#[derive(Debug, Default, Resource)]
pub struct Images {
    next_id: u64,
    images: HashMap<u64, Entry>,
    load_queue: VecDeque<(u64, LoadImage)>,
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
        let id = self.next_id();
        self.load_queue.push_back((id, source.into()));
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

fn load_images(mut images: ResMut<Images>) {
    while let Some((handle, source)) = images.load_queue.pop_front() {
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
                    format: TextureFormat::Rgba8UnormSrgb,
                    width: img.width(),
                    height: img.height(),
                    bytes: img.into_raw(),
                },
                ref_count: 1,
            },
        );
    }
}

fn update_image_handles(mut images: ResMut<Images>) {
    let images = &mut *images;

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
