use std::fmt::{self, Debug, Formatter};
use std::panic::Location;
use std::sync::Arc;

use bevy_ecs::world::World;
use slotmap::DefaultKey;

use super::Scope;

#[track_caller]
pub fn create_effect<F>(cx: &Scope, f: F)
where
    F: Fn(&World) + Send + Sync + 'static,
{
    let effect = Effect {
        f: Arc::new(f),
        signals: vec![],
        first_run: true,
        #[cfg(debug_assertions)]
        location: Location::caller(),
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
    #[cfg(debug_assertions)]
    pub location: &'static Location<'static>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EffectId(pub DefaultKey);

impl Debug for Effect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut v = f.debug_struct("Effect");

        v.field("f", &Arc::as_ptr(&self.f))
            .field("signals", &self.signals)
            .field("first_run", &self.first_run);

        #[cfg(debug_assertions)]
        v.field("location", &self.location);

        v.finish()
    }
}
