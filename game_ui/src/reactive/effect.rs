use std::fmt::{self, Debug, Formatter};
use std::panic::Location;
use std::sync::Arc;

use bevy_ecs::world::World;
use slotmap::DefaultKey;

use super::{NodeId, Scope};

#[track_caller]
pub fn create_effect<F>(cx: &Scope, f: F)
where
    F: Fn(&World) + Send + Sync + 'static,
{
    let effect = Effect {
        node: cx.id(),
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
    pub(super) node: Option<NodeId>,
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bevy_ecs::world::World;
    use parking_lot::Mutex;

    use crate::events::Events;
    use crate::reactive::{create_signal, Document};
    use crate::render::layout::LayoutTree;

    use super::super::tests::create_node;
    use super::create_effect;

    #[test]
    fn effect_call_on_creation() {
        let doc = Document::new();
        let cx = doc.root_scope();

        let value = Arc::new(Mutex::new(0));

        {
            let value = value.clone();
            create_effect(&cx, move |_| {
                *value.lock() += 1;
            });
        }

        doc.run_effects(&World::new());

        assert_eq!(*value.lock(), 1);
    }

    #[test]
    fn effect_tracks_signal() {
        let doc = Document::new();
        let cx = doc.root_scope();

        let value = Arc::new(Mutex::new(0));

        let (reader, writer) = create_signal(&cx, 0);

        {
            let value = value.clone();
            create_effect(&cx, move |_| {
                let val = reader.get();
                *value.lock() = val;
            });
        }

        doc.run_effects(&World::new());

        for _ in 0..10 {
            let val = writer.update(|val| {
                *val += 1;
                *val
            });

            doc.run_effects(&World::new());

            assert_eq!(*value.lock(), val);
        }
    }

    #[test]
    fn effect_cleanup() {
        let doc = Document::new();
        let cx = doc.root_scope();

        let mut tree = LayoutTree::new();
        let mut events = Events::new();
        let world = World::new();

        let cx = cx.push(create_node());

        create_effect(&cx, move |_| {});

        doc.run_effects(&world);
        doc.flush_node_queue(&mut tree, &mut events);

        {
            let inner = doc.inner.lock();
            assert_eq!(inner.effects.len(), 1);
        }

        cx.remove(cx.id().unwrap());

        doc.run_effects(&world);
        doc.flush_node_queue(&mut tree, &mut events);

        {
            let inner = doc.inner.lock();
            assert_eq!(inner.effects.len(), 0);
        }
    }
}
