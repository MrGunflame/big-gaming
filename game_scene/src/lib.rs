#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

mod loader;
mod model;

pub mod scene;

#[cfg(feature = "gltf")]
mod gltf;

use game_core::hierarchy::{Entity, TransformHierarchy};
use game_gltf::uri::Uri;
use game_gltf::GltfDecoder;
use game_model::{Decode, Model};
use game_render::entities::{DirectionalLightId, ObjectId, PointLightId, SpotLightId};
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SceneId(DefaultKey);

#[derive(Debug, Default)]
pub struct Scenes {
    scenes: SlotMap<DefaultKey, SceneState>,
    load_queue: VecDeque<(DefaultKey, Entity, PathBuf)>,
    queue: Arc<Mutex<Vec<(DefaultKey, Entity, Scene)>>>,
    entities: Entities,
}

#[derive(Debug, Default)]
struct Entities {
    objects: HashMap<Entity, ObjectId>,
    directional_lights: HashMap<Entity, DirectionalLightId>,
    point_lights: HashMap<Entity, PointLightId>,
    spot_lights: HashMap<Entity, SpotLightId>,

    // Parent => scene children
    children: HashMap<Entity, Vec<Entity>>,
}

impl Scenes {
    pub fn new() -> Self {
        Self {
            scenes: SlotMap::new(),
            load_queue: VecDeque::new(),
            queue: Arc::default(),
            entities: Entities::default(),
        }
    }

    pub fn insert(&mut self, entity: Entity, scene: Scene) {
        self.scenes.insert(SceneState::Ready(Some(scene), entity));
    }

    pub fn load<S>(&mut self, entity: Entity, source: S)
    where
        S: AsRef<Path>,
    {
        let id = self.scenes.insert(SceneState::Loading);
        self.load_queue
            .push_back((id, entity, source.as_ref().into()));
    }

    pub fn update(
        &mut self,
        hierarchy: &mut TransformHierarchy,
        renderer: &mut Renderer,
        pool: &TaskPool,
    ) {
        let _span = trace_span!("Scenes::update").entered();

        while let Some((key, entity, path)) = self.load_queue.pop_back() {
            let queue = self.queue.clone();
            pool.spawn(async move {
                if let Some(scene) = load_scene(path) {
                    queue.lock().unwrap().push((key, entity, scene));
                }
            });
        }

        let mut queue = self.queue.lock().unwrap();
        while let Some((key, entity, scene)) = queue.pop() {
            *self.scenes.get_mut(key).unwrap() = SceneState::Ready(Some(scene), entity);
        }
        drop(queue);

        self.scenes.retain(|_, state| match state {
            SceneState::Loading => true,
            SceneState::Ready(scene, entity) => {
                let entities =
                    scene
                        .take()
                        .unwrap()
                        .spawn(renderer, *entity, hierarchy, &mut self.entities);
                self.entities.children.insert(*entity, entities);
                false
            }
        });

        self.update_transform(hierarchy, renderer);
    }

    fn update_transform(&mut self, hierarchy: &mut TransformHierarchy, renderer: &mut Renderer) {
        // Despawn removed entities.
        self.entities.children.retain(|parent, children| {
            if !hierarchy.exists(*parent) {
                for entity in children {
                    if let Some(id) = self.entities.objects.remove(&entity) {
                        renderer.entities.objects.remove(id);
                    }

                    if let Some(id) = self.entities.directional_lights.remove(&entity) {
                        renderer.entities.directional_lights.remove(id);
                    }

                    if let Some(id) = self.entities.point_lights.remove(&entity) {
                        renderer.entities.point_lights.remove(id);
                    }

                    if let Some(id) = self.entities.spot_lights.remove(&entity) {
                        renderer.entities.spot_lights.remove(id);
                    }
                }

                false
            } else {
                true
            }
        });

        for (entity, transform) in hierarchy.iter_changed_global_transform() {
            // Not all nodes have an assocaited renderable entity.
            if let Some(id) = self.entities.objects.get(&entity) {
                let mut object = renderer.entities.objects.get_mut(*id).unwrap();
                object.transform = transform;
            }

            if let Some(id) = self.entities.directional_lights.get(&entity) {
                let mut dir_light = renderer.entities.directional_lights.get_mut(*id).unwrap();
                dir_light.transform = transform;
            }

            if let Some(id) = self.entities.point_lights.get(&entity) {
                let mut point_light = renderer.entities.point_lights.get_mut(*id).unwrap();
                point_light.transform = transform;
            }

            if let Some(id) = self.entities.spot_lights.get(&entity) {
                let mut spot_light = renderer.entities.spot_lights.get_mut(*id).unwrap();
                spot_light.transform = transform;
            }
        }
    }

    pub fn objects(&self, entity: Entity) -> Option<impl Iterator<Item = ObjectId> + '_> {
        let e = self.entities.children.get(&entity)?;
        Some(
            e.iter()
                .filter_map(|e| self.entities.objects.get(e))
                .copied(),
        )
    }
}

#[derive(Clone, Debug)]
enum SceneState {
    Loading,
    // Option so we can take.
    Ready(Option<Scene>, Entity),
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
            let mut gltf = match GltfDecoder::new(&buf) {
                Ok(gltf) => gltf,
                Err(err) => {
                    tracing::error!("failed to load GLTF file: {}", err);
                    return None;
                }
            };

            while let Some(path) = gltf.pop_source() {
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

                gltf.push_source(path, buf);
            }

            match gltf.finish() {
                Ok(gltf) => gltf.load(),
                Err(err) => {
                    tracing::error!("failed to load GLTF file: {}", err);
                    return None;
                }
            }
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
