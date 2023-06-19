use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use bevy_ecs::system::{ResMut, Resource};
use parking_lot::Mutex;

#[derive(Debug, Resource)]
pub struct Scenes {
    next_id: u64,
    scenes: HashMap<u64, Entry>,
    load_queue: VecDeque<(u64, PathBuf)>,
    events: Arc<Mutex<VecDeque<Event>>>,
}

#[derive(Debug)]
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
    data: Scene,
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
struct Scene {}

fn load_scenes(mut scenes: ResMut<Scenes>) {
    while let Some((handle, path)) = scenes.load_queue.pop_front() {
        let mut file = std::fs::File::open(path).unwrap();

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
    }
}
