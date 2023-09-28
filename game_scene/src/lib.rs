#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

mod loader;
mod model;

pub mod scene;

#[cfg(feature = "gltf")]
mod gltf;

use game_core::hierarchy::{Entity, TransformHierarchy};
use game_gltf::uri::Uri;
use game_gltf::GltfData;
use game_model::{Decode, Model};
use game_render::entities::ObjectId;
use game_render::Renderer;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use loader::LoadScene;
use scene::Scene;
use slotmap::{DefaultKey, SlotMap};

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use game_common::components::transform::Transform;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SceneId(DefaultKey);

#[derive(Debug, Default)]
pub struct Scenes {
    scenes: SlotMap<DefaultKey, SceneState>,
    nodes: HashMap<Entity, ObjectId>,
    hierarchy: TransformHierarchy,
    load_queue: VecDeque<(DefaultKey, PathBuf)>,
    queue: Arc<Mutex<Vec<(DefaultKey, Scene)>>>,
}

impl Scenes {
    pub fn new() -> Self {
        Self {
            scenes: SlotMap::new(),
            hierarchy: TransformHierarchy::new(),
            load_queue: VecDeque::new(),
            nodes: HashMap::new(),
            queue: Arc::default(),
        }
    }

    pub fn insert(&mut self, scene: Scene) -> SceneId {
        let key = self.scenes.insert(SceneState::Ready(Some(scene)));
        SceneId(key)
    }

    pub fn load<S>(&mut self, source: S) -> SceneId
    where
        S: AsRef<Path>,
    {
        let id = self.scenes.insert(SceneState::Loading);
        self.load_queue.push_back((id, source.as_ref().into()));
        SceneId(id)
    }

    pub fn update(&mut self, renderer: &mut Renderer, pool: &TaskPool) {
        let _span = trace_span!("Scenes::update").entered();

        while let Some((key, path)) = self.load_queue.pop_back() {
            let queue = self.queue.clone();
            pool.spawn(async move {
                if let Some(scene) = load_scene(path) {
                    queue.lock().unwrap().push((key, scene));
                }
            });
        }

        let mut queue = self.queue.lock().unwrap();
        while let Some((key, scene)) = queue.pop() {
            *self.scenes.get_mut(key).unwrap() = SceneState::Ready(Some(scene));
        }
        drop(queue);

        for state in self.scenes.values_mut() {
            match state {
                SceneState::Loading => (),
                SceneState::Ready(scene) => {
                    let id =
                        scene
                            .take()
                            .unwrap()
                            .spawn(renderer, &mut self.hierarchy, &mut self.nodes);
                    *state = SceneState::Spawned(id);
                }
                SceneState::Spawned(_) => (),
            }
        }

        self.update_transform(renderer);
    }

    fn update_transform(&mut self, renderer: &mut Renderer) {
        self.hierarchy.compute_transform();

        for (entity, transform) in self.hierarchy.iter_changed_global_transform() {
            // Not all entities have an render object associated.
            if let Some(id) = self.nodes.get(&entity) {
                let mut object = renderer.entities.objects.get_mut(*id).unwrap();
                object.transform = transform;
            }
        }
    }

    pub fn set_transform(&mut self, id: SceneId, transform: Transform) {
        let scene = match self.scenes.get(id.0) {
            Some(SceneState::Spawned(id)) => id,
            _ => return,
        };

        self.hierarchy.set(*scene, transform);
    }

    pub fn get_transform(&self, id: SceneId) -> Option<Transform> {
        let scene = match self.scenes.get(id.0) {
            Some(SceneState::Spawned(id)) => id,
            _ => return None,
        };

        self.hierarchy.get(*scene)
    }

    pub fn objects(&self, id: SceneId) -> Option<impl Iterator<Item = ObjectId>> {
        let scene = match self.scenes.get(id.0)? {
            SceneState::Spawned(id) => id,
            _ => return None,
        };

        let mut nodes = vec![];
        for node in self.hierarchy.children(*scene).unwrap() {
            if let Some(obj) = self.nodes.get(&node) {
                nodes.push(*obj);
            }
        }

        Some(nodes.into_iter())
    }
}

#[derive(Clone, Debug)]
enum SceneState {
    Loading,
    // Option so we can take.
    Ready(Option<Scene>),
    Spawned(Entity),
}

fn load_scene(path: PathBuf) -> Option<Scene> {
    let _span = trace_span!("load_scene").entered();

    let uri = Uri::from(path);

    let mut file = match File::open(uri.as_path()) {
        Ok(file) => file,
        Err(err) => {
            tracing::error!("failed to load scene from {:?}: {}", uri, err);
            return None;
        }
    };

    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();

    let scene = match detect_format(&buf) {
        Some(SceneFormat::Model) => {
            let data = match Model::decode(&buf[..]) {
                Ok(data) => data,
                Err(err) => {
                    tracing::error!("failed to load model: {:?}", err);
                    return None;
                }
            };

            data.load()
        }
        Some(SceneFormat::Gltf) => {
            let mut gltf = match GltfData::new(&buf) {
                Ok(gltf) => gltf,
                Err(err) => {
                    tracing::error!("failed to load GLTF file: {}", err);
                    return None;
                }
            };

            while let Some(path) = gltf.queue.pop() {
                let mut uri = uri.clone();
                uri.push(&path);
                let mut file = match std::fs::File::open(uri.as_path()) {
                    Ok(file) => file,
                    Err(err) => {
                        tracing::error!("failed to load file for GLTF: {}", err);
                        return None;
                    }
                };

                let mut buf = Vec::new();
                if let Err(err) = file.read_to_end(&mut buf) {
                    tracing::error!("failed to load file for GLTF: {}", err);
                    return None;
                }

                gltf.insert(path, buf);
            }

            gltf.create_unchecked().load()
        }
        None => {
            tracing::error!("cannot detect scene format");
            return None;
        }
    };

    Some(scene)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum SceneFormat {
    Model,
    Gltf,
}

/// Attempt to detect the file format.
fn detect_format(buf: &[u8]) -> Option<SceneFormat> {
    if buf.starts_with(&game_model::MAGIC) {
        return Some(SceneFormat::Model);
    }

    // Starts with 'glTF' for binary format, or a JSON object.
    if buf.starts_with(&[b'g', b'l', b'T', b'F']) || buf.starts_with(&[b'{']) {
        return Some(SceneFormat::Gltf);
    }

    None
}
