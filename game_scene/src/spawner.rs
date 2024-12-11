use std::collections::{HashMap, VecDeque};

use game_common::collections::arena::{self, Arena};
use game_common::components::Transform;
use game_render::Renderer;
use game_tasks::{Task, TaskPool};
use game_tracing::trace_span;

use crate::load_from_bytes;
use crate::scene::Scene;
use crate::scene2::{SceneResources, SpawnedScene};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SceneId(arena::Key);

#[derive(Debug, Default)]
pub struct SceneSpawner {
    instances: Arena<Instance>,
    scenes: Arena<SceneData>,
    events: VecDeque<Event>,
    tasks: HashMap<SceneId, SceneLoadState>,
}

impl SceneSpawner {
    pub fn insert(&mut self, data: &[u8]) -> SceneId {
        let _span = trace_span!("SceneSpawner::insert").entered();

        let id = SceneId(self.scenes.insert(SceneData::Queued));
        self.events.push_back(Event::SpawnScene(data.to_vec(), id));
        id
    }

    // TODO: Remove this
    pub fn insert_from_file(&mut self, path: &str) -> SceneId {
        match std::fs::read(path) {
            Ok(buf) => self.insert(&buf),
            Err(err) => {
                tracing::error!("loading from file failed: {:?}", err);
                SceneId(self.scenes.insert(SceneData::Failed))
            }
        }
    }

    pub fn remove(&mut self, id: SceneId) {
        let _span = trace_span!("SceneSpawner::remove").entered();

        if self.scenes.contains_key(id.0) {
            self.events.push_back(Event::DestroyScene(id));
        }
    }

    pub fn spawn(&mut self, scene: SceneId) -> InstanceId {
        let _span = trace_span!("SceneSpawner::spawn").entered();
        let id = InstanceId(self.instances.insert(Instance {
            scene,
            state: InstanceState::Loading,
            transform: Transform::default(),
        }));
        self.events.push_back(Event::SpawnInstance(id, scene));
        id
    }

    pub fn despawn(&mut self, id: InstanceId) {
        let _span = trace_span!("SceneSpawner::despawn").entered();

        if self.instances.contains_key(id.0) {
            self.events.push_back(Event::DestroyInstance(id));
        }
    }

    pub fn scene_of_instance(&self, instance: InstanceId) -> SceneId {
        self.instances.get(instance.0).unwrap().scene
    }

    pub fn update(
        &mut self,
        pool: &TaskPool,
        renderer: &mut Renderer,
        scene_id: game_render::entities::SceneId,
    ) {
        let _span = trace_span!("SceneSpaner::update").entered();

        while let Some(event) = self.events.pop_front() {
            match event {
                Event::SpawnScene(data, scene) => {
                    let task = pool.spawn(async move {
                        match load_from_bytes(&data) {
                            Ok(scene) => Some(scene),
                            Err(err) => {
                                tracing::error!("failed to load scene: {:?}", err);
                                None
                            }
                        }
                    });

                    self.tasks.insert(
                        scene,
                        SceneLoadState {
                            task,
                            deferred_instances: Vec::new(),
                        },
                    );
                }
                Event::SpawnInstance(instance, scene) => {
                    match self.scenes.get(scene.0).unwrap() {
                        SceneData::Loaded(scene, resources) => {
                            let instance = self.instances.get_mut(instance.0).unwrap();

                            let mut state = scene.instantiate(resources, renderer, scene_id);
                            state.set_transform(instance.transform);
                            state.compute_transform();

                            for (key, object) in &mut state.entities {
                                let global_transform = *state.global_transform.get(key).unwrap();

                                // Recreate the object once the transform changes.
                                // The renderer does not support updating existing
                                // objects.
                                object.object.transform = global_transform;
                                renderer.resources().objects().remove(object.id);
                                object.id = renderer.resources().objects().insert(object.object);
                            }

                            instance.state = InstanceState::Spawned(state);
                        }
                        SceneData::Queued => {
                            // Defer instace creation until the scene is loaded.
                            self.tasks
                                .get_mut(&scene)
                                .unwrap()
                                .deferred_instances
                                .push(instance);
                        }
                        // Skip instance creation if the scene is invalid.
                        // In this case all operations on the instance are
                        // just nops.
                        SceneData::Failed => {}
                    }
                }
                Event::DestroyScene(scene) => {
                    // The scene must not have any instances refering to it.
                    if cfg!(debug_assertions) {
                        for instance in self.instances.values() {
                            assert_ne!(instance.scene, scene);
                        }
                    }

                    self.scenes.remove(scene.0);
                    self.tasks.remove(&scene);
                }
                Event::DestroyInstance(instance) => {
                    let Some(instance) = self.instances.remove(instance.0) else {
                        continue;
                    };

                    if let InstanceState::Spawned(state) = instance.state {
                        state.despawn(renderer);
                    }
                }
                Event::SetTransform(instance, transform) => {
                    let instance = self.instances.get_mut(instance.0).unwrap();
                    // Updating the transform of objects can be expensive.
                    // Avoid unless the transform value actually changed.
                    if instance.transform == transform {
                        continue;
                    }

                    instance.transform = transform;
                    match &mut instance.state {
                        InstanceState::Loading => {
                            // If the scene is not yet loaded the transform update
                            // needs to be performed once the scene is spawned.
                            // The instance will use the `instance.transform` value
                            // that we have just written.
                        }
                        InstanceState::Spawned(state) => {
                            state.set_transform(transform);
                            state.compute_transform();

                            for (key, object) in &mut state.entities {
                                let global_transform = *state.global_transform.get(key).unwrap();

                                // Recreate the object once the transform changes.
                                // The renderer does not support updating existing
                                // objects.
                                object.object.transform = global_transform;
                                renderer.resources().objects().remove(object.id);
                                object.id = renderer.resources().objects().insert(object.object);
                            }
                        }
                    }
                }
            }
        }

        self.tasks
            .retain(|scene, state| match state.task.get_output() {
                Some(Some(mut output)) => {
                    let res = output.setup_materials(renderer);
                    *self.scenes.get_mut(scene.0).unwrap() = SceneData::Loaded(output, res);

                    for instance in state.deferred_instances.drain(..) {
                        self.events
                            .push_back(Event::SpawnInstance(instance, *scene));
                    }

                    false
                }
                Some(None) => {
                    *self.scenes.get_mut(scene.0).unwrap() = SceneData::Failed;
                    false
                }
                None => true,
            });
    }

    pub fn set_transform(&mut self, instance: InstanceId, transform: Transform) {
        debug_assert!(self.instances.contains_key(instance.0));
        self.events
            .push_back(Event::SetTransform(instance, transform));
    }
}

#[derive(Debug)]
struct Instance {
    scene: SceneId,
    state: InstanceState,
    transform: Transform,
}

#[derive(Debug)]
enum InstanceState {
    Spawned(SpawnedScene),
    Loading,
}

#[derive(Debug)]
enum SceneData {
    Loaded(Scene, SceneResources),
    Queued,
    Failed,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InstanceId(arena::Key);

#[derive(Clone, Debug)]
enum Event {
    SpawnScene(Vec<u8>, SceneId),
    SpawnInstance(InstanceId, SceneId),
    DestroyScene(SceneId),
    DestroyInstance(InstanceId),
    SetTransform(InstanceId, Transform),
}

#[derive(Debug)]
struct SceneLoadState {
    task: Task<Option<Scene>>,
    deferred_instances: Vec<InstanceId>,
}
