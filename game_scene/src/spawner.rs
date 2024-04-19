use std::collections::HashMap;
use std::path::{Path, PathBuf};

use game_common::collections::arena::{self, Arena};
use game_common::components::Transform;
use game_render::Renderer;
use game_tasks::{Task, TaskPool};
use game_tracing::trace_span;

use crate::load_scene;
use crate::scene::Scene;
use crate::scene2::{SceneResources, SpawnedScene};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SceneId(arena::Key);

#[derive(Debug, Default)]
pub struct SceneSpawner {
    instances: Arena<Instance>,
    queued_instances: Vec<SceneId>,
    scenes: HashMap<PathBuf, SceneData>,
}

impl SceneSpawner {
    pub fn spawn<S>(&mut self, source: S) -> SceneId
    where
        S: AsRef<Path>,
    {
        let id = SceneId(self.instances.insert(Instance {
            path: source.as_ref().to_owned(),
            state: InstanceState::Loading,
        }));
        self.queued_instances.push(id);

        let scene = self
            .scenes
            .entry(source.as_ref().to_owned())
            .or_insert_with(|| SceneData {
                count: 0,
                state: SceneDataState::Queued,
            });
        scene.count += 1;

        id
    }

    pub fn update(&mut self, pool: &TaskPool, renderer: &mut Renderer) {
        let _span = trace_span!("SceneSpaner::update").entered();

        self.queued_instances.retain(|id| {
            let instance = self.instances.get_mut(id.0).unwrap();
            let scene = self.scenes.get_mut(&instance.path).unwrap();

            match &mut scene.state {
                SceneDataState::Loaded(scene, res) => {
                    let spawned_scene = scene.instantiate(res, renderer);
                    instance.state = InstanceState::Spawned(spawned_scene);

                    false
                }
                SceneDataState::Loading(task) => {
                    // FIXME: We're actually checking for every instance, which means
                    // it is possible for the same task handle to be checked polled
                    // multiple times without effect.
                    if let Some(output) = task.get_output() {
                        match output {
                            Some(mut output) => {
                                let res = output.setup_materials(renderer);

                                // Instantiate the instance has caused the initial load of the
                                // asset. This allows us to skip delaying the creation of the
                                // instance until the next update.
                                let spawned_scene = output.instantiate(&res, renderer);
                                instance.state = InstanceState::Spawned(spawned_scene);

                                scene.state = SceneDataState::Loaded(output, res);
                            }
                            None => scene.state = SceneDataState::LoadingFailed,
                        }

                        false
                    } else {
                        true
                    }
                }
                SceneDataState::LoadingFailed => false,
                SceneDataState::Queued => {
                    let path = instance.path.clone();
                    let task = pool.spawn(async move { load_scene(path) });
                    scene.state = SceneDataState::Loading(task);

                    true
                }
            }
        });
    }

    pub fn despawn(&mut self, renderer: &mut Renderer, id: SceneId) {
        tracing::trace!("despawn scene {:?}", id);

        if let Some(instance) = self.instances.remove(id.0) {
            if let InstanceState::Spawned(spawned_scene) = instance.state {
                spawned_scene.despawn(renderer);
            }

            if let Some(scene) = self.scenes.get_mut(&instance.path) {
                scene.count -= 1;

                if scene.count == 0 {
                    self.scenes.remove(&instance.path);
                }
            }
        }
    }

    pub fn set_transform(&mut self, renderer: &mut Renderer, transform: Transform, id: SceneId) {
        let instance = self.instances.get_mut(id.0).unwrap();
        match &mut instance.state {
            InstanceState::Loading => {
                // What to do if scene is not yet loaded?
            }
            InstanceState::Spawned(scene) => {
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
struct Instance {
    path: PathBuf,
    state: InstanceState,
}

#[derive(Debug)]
enum InstanceState {
    Spawned(SpawnedScene),
    Loading,
}

#[derive(Debug)]
struct SceneData {
    /// Number of instances referencing the scene data.
    count: usize,
    state: SceneDataState,
}

#[derive(Debug)]
enum SceneDataState {
    Loaded(Scene, SceneResources),
    Loading(Task<Option<Scene>>),
    LoadingFailed,
    Queued,
}
