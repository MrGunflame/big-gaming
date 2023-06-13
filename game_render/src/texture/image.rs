use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::path::{Path, PathBuf};

use bevy_app::{App, Plugin};
use bevy_ecs::system::{ResMut, Resource};
use game_asset::{Asset, LoadAsset};
use image::load_from_memory;

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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextureFormat {
    Rgba8Unorm,
    Rgba8UnormSrgb,
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
    }
}

#[derive(Clone, Debug, Default, Resource)]
pub struct Images {
    next_id: u64,
    images: HashMap<ImageHandle, Image>,
    load_queue: VecDeque<(ImageHandle, PathBuf)>,
}

impl Images {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            images: HashMap::new(),
            load_queue: VecDeque::new(),
        }
    }

    pub fn insert(&mut self, image: Image) -> ImageHandle {
        let id = self.next_id();
        self.images.insert(id, image);
        id
    }

    pub fn get(&self, handle: ImageHandle) -> Option<&Image> {
        self.images.get(&handle)
    }

    pub fn remove(&mut self, handle: ImageHandle) -> Option<Image> {
        self.images.remove(&handle)
    }

    fn next_id(&mut self) -> ImageHandle {
        let id = self.next_id;
        self.next_id += 1;
        ImageHandle(id)
    }

    pub fn load<P>(&mut self, path: P) -> ImageHandle
    where
        P: AsRef<Path>,
    {
        let id = self.next_id();
        self.load_queue.push_back((id, path.as_ref().into()));
        id
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImageHandle(u64);

fn load_images(mut images: ResMut<Images>) {
    while let Some((handle, path)) = images.load_queue.pop_front() {
        let mut file = std::fs::File::open(path).unwrap();

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();

        let img = image::load_from_memory(&buf).unwrap().to_rgba8();

        images.images.insert(
            handle,
            Image {
                format: TextureFormat::Rgba8UnormSrgb,
                width: img.width(),
                height: img.height(),
                bytes: img.into_raw(),
            },
        );
    }
}
