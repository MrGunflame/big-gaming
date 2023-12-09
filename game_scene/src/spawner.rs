use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use game_render::Renderer;
use game_tasks::TaskPool;
use game_tracing::trace_span;

use crate::format::SceneRoot;
use crate::load_scene;
use crate::scene2::{Key, SceneGraph};

#[derive(Debug, Default)]
pub struct SceneSpawner {
    queue: VecDeque<(Key, PathBuf)>,
    scenes_to_spawn: Arc<Mutex<VecDeque<(Key, EitherScene)>>>,
}

impl SceneSpawner {
    pub fn spawn<S>(&mut self, parent: Key, source: S)
    where
        S: AsRef<Path>,
    {
        self.queue
            .push_back((parent, source.as_ref().to_path_buf()));
    }

    pub fn insert(&mut self, parent: Key, scene: impl Into<EitherScene>) {
        self.scenes_to_spawn
            .lock()
            .unwrap()
            .push_back((parent, scene.into()));
    }

    pub fn update(
        &mut self,
        graph: &mut SceneGraph,
        pool: &TaskPool,
        mut renderer: Option<&mut Renderer>,
    ) {
        let _span = trace_span!("SceneSpaner::update").entered();

        while let Some((key, path)) = self.queue.pop_front() {
            let queue = self.scenes_to_spawn.clone();
            pool.spawn(async move {
                if let Some(scene) = load_scene(path) {
                    queue.lock().unwrap().push_back((key, scene.into()));
                }
            });
        }

        let mut queue = self.scenes_to_spawn.lock().unwrap();
        while let Some((parent, scene)) = queue.pop_front() {
            scene.spawn(&mut renderer, parent, graph);
        }
    }
}

#[derive(Clone, Debug)]
pub enum EitherScene {
    A(SceneRoot),
    B(crate::scene::Scene),
}

impl EitherScene {
    fn spawn(self, renderer: &mut Option<&mut Renderer>, parent: Key, graph: &mut SceneGraph) {
        match self {
            Self::A(s) => s.spawn(renderer, parent, graph),
            Self::B(s) => s.spawn(renderer, parent, graph),
        }
    }
}

impl From<SceneRoot> for EitherScene {
    fn from(value: SceneRoot) -> Self {
        Self::A(value)
    }
}

impl From<crate::scene::Scene> for EitherScene {
    fn from(value: crate::scene::Scene) -> Self {
        Self::B(value)
    }
}
