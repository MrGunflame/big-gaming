use std::sync::Arc;

use bevy_ecs::world::World;

use super::{NodeId, Scope};

pub fn create_effect<F>(cx: &Scope, f: F)
where
    F: Fn(&World) + Send + Sync + 'static,
{
    let mut effect = Effect { f: Arc::new(f) };

    let mut doc = cx.document.inner.lock();
    let mut node = doc.nodes.get_mut(cx.id.0).unwrap();

    node.effects.push(effect);
}

#[derive(Clone)]
pub(super) struct Effect {
    pub(super) f: Arc<dyn Fn(&World) + Send + Sync + 'static>,
}
