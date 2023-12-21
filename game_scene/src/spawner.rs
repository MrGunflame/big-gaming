use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use game_common::collections::arena::{self, Arena};
use game_common::components::transform::Transform;
use game_render::Renderer;
use game_tasks::TaskPool;
use game_tracing::trace_span;

use crate::format::SceneRoot;
use crate::load_scene;
use crate::scene2::{Key, SpawnedScene};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SceneId(arena::Key);

#[derive(Debug, Default)]
pub struct SceneSpawner {
    queue: VecDeque<(SceneId, PathBuf)>,
    scenes_to_spawn: Arc<Mutex<VecDeque<(SceneId, crate::scene::Scene)>>>,
    scenes: Arena<SceneState>,
}

impl SceneSpawner {
    pub fn spawn<S>(&mut self, source: S) -> SceneId
    where
        S: AsRef<Path>,
    {
        let id = SceneId(self.scenes.insert(SceneState::Loading));
        self.queue.push_back((id, source.as_ref().to_path_buf()));
        id
    }

    pub fn insert(&mut self, scene: crate::scene::Scene) -> SceneId {
        let id = SceneId(self.scenes.insert(SceneState::Loading));
        self.scenes_to_spawn.lock().unwrap().push_back((id, scene));
        id
    }

    pub fn update(&mut self, pool: &TaskPool, renderer: &mut Renderer) {
        let _span = trace_span!("SceneSpaner::update").entered();

        while let Some((id, path)) = self.queue.pop_front() {
            let queue = self.scenes_to_spawn.clone();
            pool.spawn(async move {
                if let Some(scene) = load_scene(path) {
                    queue.lock().unwrap().push_back((id, scene));
                }
            });
        }

        let mut queue = self.scenes_to_spawn.lock().unwrap();
        while let Some((id, scene)) = queue.pop_front() {
            let spawned_scene = scene.spawn(renderer);
            if let Some(scene) = self.scenes.get_mut(id.0) {
                *scene = SceneState::Spawned(spawned_scene);
            } else {
                // Already despawned.
                spawned_scene.destroy_resources(renderer);
            }
        }
    }

    pub fn despawn(&mut self, renderer: &mut Renderer, id: SceneId) {
        let scene = self.scenes.remove(id.0).unwrap();
        match scene {
            SceneState::Loading => {}
            SceneState::Spawned(scene) => {
                for (_, id) in &scene.entities {
                    renderer.entities.objects.remove(*id);
                }

                scene.destroy_resources(renderer);
            }
        }
    }

    pub fn set_transform(&mut self, renderer: &mut Renderer, transform: Transform, id: SceneId) {
        let scene = self.scenes.get_mut(id.0).unwrap();
        match scene {
            SceneState::Loading => {
                // What to do if scene is not yet loaded?
            }
            SceneState::Spawned(scene) => {
                scene.set_transform(transform);
                scene.compute_transform();

                for (key, id) in &scene.entities {
                    let global_transform = *scene.global_transform.get(key).unwrap();

                    let mut object = renderer.entities.objects.get_mut(*id).unwrap();
                    object.transform = global_transform;
                }
            }
        }
    }
}

#[derive(Debug)]
enum SceneState {
    Spawned(SpawnedScene),
    Loading,
}
