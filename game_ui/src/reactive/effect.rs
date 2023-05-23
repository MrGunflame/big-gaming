use std::sync::Arc;

use bevy_ecs::world::World;
use slotmap::DefaultKey;

use super::{NodeId, Scope};

pub fn create_effect<F>(cx: &Scope, f: F)
where
    F: Fn(&World) + Send + Sync + 'static,
{
    let effect = Effect {
        f: Arc::new(f),
        signals: vec![],
        first_run: true,
    };

    let mut doc = cx.document.inner.lock();

    let key = doc.effects.insert(effect);

    // Immediately queue the effect for execution.
    doc.effect_queue.push(EffectId(key));
}

#[derive(Clone)]
pub(super) struct Effect {
    pub(super) f: Arc<dyn Fn(&World) + Send + Sync + 'static>,
    pub(super) signals: Vec<DefaultKey>,
    pub(super) first_run: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EffectId(pub DefaultKey);
