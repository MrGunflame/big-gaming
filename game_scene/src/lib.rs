#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

mod scene;

#[cfg(feature = "gltf")]
mod gltf;

use game_gltf::uri::Uri;
pub use scene::SceneBundle;

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::Component;
use bevy_ecs::system::{ResMut, Resource};
use game_asset::{Assets, Handle};
use game_common::components::transform::Transform;
use game_gltf::GltfData;
use game_render::mesh::Mesh;
use game_render::pbr::PbrMaterial;
use game_render::texture::Images;
use gltf::gltf_to_scene;
use parking_lot::Mutex;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Scenes::default());
        app.add_system(load_scenes);
        app.add_system(update_scene_handles);
        app.add_system(scene::spawn_scene);
    }
}

#[derive(Debug, Default, Resource)]
pub struct Scenes {
    next_id: u64,
    scenes: HashMap<u64, Entry>,
    load_queue: VecDeque<(u64, PathBuf)>,
    events: Arc<Mutex<VecDeque<Event>>>,
}

impl Scenes {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            scenes: HashMap::new(),
            load_queue: VecDeque::new(),
            events: Arc::default(),
        }
    }

    pub fn insert(&mut self, scene: Scene) -> SceneHandle {
        let id = self.next_id();
        self.scenes.insert(
            id,
            Entry {
                data: Some(scene),
                ref_count: 1,
            },
        );

        SceneHandle {
            id,
            events: self.events.clone(),
        }
    }

    pub fn get(&self, handle: &SceneHandle) -> Option<&Scene> {
        self.scenes
            .get(&handle.id)
            .map(|entry| entry.data.as_ref())
            .flatten()
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn load<S>(&mut self, source: S) -> SceneHandle
    where
        S: AsRef<Path>,
    {
        let id = self.next_id();
        self.scenes.insert(
            id,
            Entry {
                data: None,
                ref_count: 1,
            },
        );
        self.load_queue.push_back((id, source.as_ref().into()));
        SceneHandle {
            id,
            events: self.events.clone(),
        }
    }
}

#[derive(Debug, Component)]
pub struct SceneHandle {
    id: u64,
    events: Arc<Mutex<VecDeque<Event>>>,
}

impl Clone for SceneHandle {
    fn clone(&self) -> Self {
        self.events.lock().push_back(Event::Clone(self.id));

        Self {
            id: self.id,
            events: self.events.clone(),
        }
    }
}

impl Drop for SceneHandle {
    fn drop(&mut self) {
        self.events.lock().push_back(Event::Drop(self.id));
    }
}

#[derive(Debug)]
struct Entry {
    data: Option<Scene>,
    ref_count: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum SceneKind {
    Gltf,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Event {
    Drop(u64),
    Clone(u64),
}

#[derive(Debug)]
pub struct Scene {
    pub nodes: Vec<Node>,
}

#[derive(Debug)]
pub struct Node {
    pub mesh: Handle<Mesh>,
    pub material: Handle<PbrMaterial>,
    pub transform: Transform,
}

fn load_scenes(
    mut scenes: ResMut<Scenes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PbrMaterial>>,
    mut images: ResMut<Images>,
) {
    while let Some((handle, path)) = scenes.load_queue.pop_front() {
        let uri = Uri::from(path);

        let mut file = match File::open(uri.as_path()) {
            Ok(file) => file,
            Err(err) => {
                tracing::error!("failed to load scene from {:?}: {}", uri, err);
                continue;
            }
        };

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();

        let mut gltf = GltfData::new(&buf).unwrap();
        while let Some(path) = gltf.queue.pop() {
            let mut uri = uri.clone();
            uri.push(&path);
            let mut file = std::fs::File::open(uri.as_path()).unwrap();

            let mut buf = Vec::new();
            file.read_to_end(&mut buf).unwrap();

            gltf.insert(path, buf);
        }

        let data = gltf_to_scene(
            gltf.create_unchecked(),
            &mut meshes,
            &mut materials,
            &mut images,
        );

        // If `None` all handles are already dropped.
        if let Some(entry) = scenes.scenes.get_mut(&handle) {
            entry.data = Some(data);
        }
    }
}

fn update_scene_handles(mut scenes: ResMut<Scenes>) {
    let scenes = &mut *scenes;

    let mut events = scenes.events.lock();
    while let Some(event) = events.pop_front() {
        match event {
            Event::Clone(id) => {
                let entry = scenes.scenes.get_mut(&id).unwrap();
                entry.ref_count += 1;
            }
            Event::Drop(id) => {
                let entry = scenes.scenes.get_mut(&id).unwrap();
                entry.ref_count -= 1;

                if entry.ref_count == 0 {
                    scenes.scenes.remove(&id);
                }
            }
        }
    }
}
