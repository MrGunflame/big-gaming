use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use game_render::Renderer;
use game_tasks::TaskPool;
use game_tracing::trace_span;

use crate::load_scene;
use crate::scene::Scene;
use crate::scene2::{Key, SceneGraph};

#[derive(Debug, Default)]
pub struct SceneSpawner {
    queue: VecDeque<(Key, PathBuf)>,
    scenes_to_spawn: Arc<Mutex<VecDeque<(Key, Scene)>>>,
}

impl SceneSpawner {
    pub fn spawn<S>(&mut self, parent: Key, source: S)
    where
        S: AsRef<Path>,
    {
        self.queue
            .push_back((parent, source.as_ref().to_path_buf()));
    }

    pub fn insert(&mut self, parent: Key, scene: Scene) {
        self.scenes_to_spawn
            .lock()
            .unwrap()
            .push_back((parent, scene));
    }

    pub fn update(&mut self, graph: &mut SceneGraph, pool: &TaskPool, renderer: &mut Renderer) {
        let _span = trace_span!("SceneSpaner::update").entered();

        while let Some((key, path)) = self.queue.pop_front() {
            let queue = self.scenes_to_spawn.clone();
            pool.spawn(async move {
                if let Some(scene) = load_scene(path) {
                    queue.lock().unwrap().push_back((key, scene));
                }
            });
        }

        let mut queue = self.scenes_to_spawn.lock().unwrap();
        while let Some((parent, scene)) = queue.pop_front() {
            scene.spawn(renderer, parent, graph);
        }
    }
}
