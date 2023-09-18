#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

mod model;
mod scene;

#[cfg(feature = "gltf")]
mod gltf;

use game_core::hierarchy::{Entity, TransformHierarchy};
use game_gltf::uri::Uri;
use game_model::{Decode, Model};
use game_render::entities::ObjectId;
use game_render::Renderer;
use game_tracing::trace_span;
use scene::spawn_scene;
use slotmap::{DefaultKey, SlotMap};

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use game_asset::{Assets, Handle};
use game_common::components::transform::Transform;
use game_gltf::GltfData;
use game_render::mesh::Mesh;
use game_render::pbr::PbrMaterial;
use game_render::texture::Images;
use gltf::gltf_to_scene;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScenePlugin;

pub struct SceneId(DefaultKey);

#[derive(Debug, Default)]
pub struct Scenes {
    scenes: SlotMap<DefaultKey, SceneState>,
    nodes: HashMap<Entity, ObjectId>,
    hierarchy: TransformHierarchy,
    load_queue: VecDeque<(DefaultKey, PathBuf)>,
}

impl Scenes {
    pub fn new() -> Self {
        Self {
            scenes: SlotMap::new(),
            hierarchy: TransformHierarchy::new(),
            load_queue: VecDeque::new(),
            nodes: HashMap::new(),
        }
    }

    pub fn insert(&mut self, scene: Scene) -> SceneId {
        let key = self.scenes.insert(SceneState::Ready(scene));
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

    pub fn update(&mut self, renderer: &mut Renderer) {
        let _span = trace_span!("Scenes::update").entered();

        load_scenes(
            self,
            &mut renderer.meshes,
            &mut renderer.materials,
            &mut renderer.images,
        );

        for state in self.scenes.values_mut() {
            match state {
                SceneState::Loading => (),
                SceneState::Ready(scene) => {
                    let id = spawn_scene(scene, renderer, &mut self.hierarchy, &mut self.nodes);
                    *state = SceneState::Spawned(id);
                }
                SceneState::Spawned(_) => (),
            }
        }

        self.update_transform(renderer);
    }

    fn update_transform(&mut self, renderer: &mut Renderer) {
        self.hierarchy.compute_transform();

        for (entity, transform) in self.hierarchy.iter_changed_transform() {
            // Not all entities have an render object associated.
            if let Some(id) = self.nodes.get(&entity) {
                let object = renderer.entities.objects().get_mut(*id).unwrap();
                object.transform = transform;
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub transform: Transform,
    pub nodes: Vec<Node>,
}

#[derive(Clone, Debug)]
enum SceneState {
    Loading,
    Ready(Scene),
    Spawned(Entity),
}

#[derive(Clone, Debug)]
pub struct Node {
    pub mesh: Handle<Mesh>,
    pub material: Handle<PbrMaterial>,
    pub transform: Transform,
}

fn load_scenes(
    scenes: &mut Scenes,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<PbrMaterial>,
    images: &mut Images,
) {
    'out: while let Some((handle, path)) = scenes.load_queue.pop_front() {
        let _span = trace_span!("load_scene").entered();

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

        let scene = match detect_format(&buf) {
            Some(SceneFormat::Model) => {
                let data = match Model::decode(&buf[..]) {
                    Ok(data) => data,
                    Err(err) => {
                        tracing::error!("failed to load model: {:?}", err);
                        continue;
                    }
                };

                model::model_to_scene(data, meshes, materials, images)
            }
            Some(SceneFormat::Gltf) => {
                let mut gltf = match GltfData::new(&buf) {
                    Ok(gltf) => gltf,
                    Err(err) => {
                        tracing::error!("failed to load GLTF file: {}", err);
                        continue;
                    }
                };

                while let Some(path) = gltf.queue.pop() {
                    let mut uri = uri.clone();
                    uri.push(&path);
                    let mut file = match std::fs::File::open(uri.as_path()) {
                        Ok(file) => file,
                        Err(err) => {
                            tracing::error!("failed to load file for GLTF: {}", err);
                            continue 'out;
                        }
                    };

                    let mut buf = Vec::new();
                    if let Err(err) = file.read_to_end(&mut buf) {
                        tracing::error!("failed to load file for GLTF: {}", err);
                        continue 'out;
                    }

                    gltf.insert(path, buf);
                }

                gltf_to_scene(gltf.create_unchecked(), meshes, materials, images)
            }
            None => {
                tracing::error!("cannot detect scene format");
                continue;
            }
        };

        *scenes.scenes.get_mut(handle).unwrap() = SceneState::Ready(scene);
    }
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
