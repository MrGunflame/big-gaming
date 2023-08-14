use std::fmt::{self, Debug, Formatter};
use std::panic::Location;
use std::sync::Arc;

use slotmap::{new_key_type, DefaultKey};

use super::{NodeId, Scope};

#[track_caller]
pub fn create_effect<F>(cx: &Scope, f: F)
where
    F: Fn() + Send + Sync + 'static,
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
    let mut rt = cx.document.runtime.inner.lock();

    let key = rt.effects.insert(effect);
    doc.effects.insert(key);

    tracing::trace!("creating Effect({:?}) at {}", key, Location::caller());

    // Immediately queue the effect for execution.
    rt.effect_queue.insert(key);
}

#[derive(Clone)]
pub(super) struct Effect {
    pub(super) node: Option<NodeId>,
    pub(super) f: Arc<dyn Fn() + Send + Sync + 'static>,
    pub(super) signals: Vec<DefaultKey>,
    pub(super) first_run: bool,
    #[cfg(debug_assertions)]
    pub location: &'static Location<'static>,
}

new_key_type! {
    pub struct EffectId;
}

impl Debug for Effect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut v = f.debug_struct("Effect");

        v.field("node", &self.node)
            .field("f", &Arc::as_ptr(&self.f))
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

    use parking_lot::Mutex;

    use crate::events::Events;
    use crate::reactive::{create_signal, Document, Runtime};
    use crate::render::layout::LayoutTree;

    use super::super::tests::create_node;
    use super::create_effect;

    #[test]
    fn effect_call_on_creation() {
        let rt = Runtime::new();
        let doc = Document::new(rt);
        let cx = doc.root_scope();

        let value = Arc::new(Mutex::new(0));

        {
            let value = value.clone();
            create_effect(&cx, move || {
                *value.lock() += 1;
            });
        }

        doc.run_effects();

        assert_eq!(*value.lock(), 1);
    }

    #[test]
    fn effect_tracks_signal() {
        let rt = Runtime::new();
        let doc = Document::new(rt);
        let cx = doc.root_scope();

        let value = Arc::new(Mutex::new(0));

        let (reader, writer) = create_signal(&cx, 0);

        {
            let value = value.clone();
            create_effect(&cx, move || {
                let val = reader.get();
                *value.lock() = val;
            });
        }

        doc.run_effects();

        for _ in 0..10 {
            let val = writer.update(|val| {
                *val += 1;
                *val
            });

            doc.run_effects();

            assert_eq!(*value.lock(), val);
        }
    }

    #[test]
    fn effect_untracked_signal() {
        let rt = Runtime::new();
        let doc = Document::new(rt);
        let cx = doc.root_scope();

        let value = Arc::new(Mutex::new(0));

        let (reader, writer) = create_signal(&cx, 0);

        {
            let value = value.clone();
            create_effect(&cx, move || {
                let val = reader.get_untracked();
                *value.lock() = val;
            });
        }

        doc.run_effects();

        for _ in 0..10 {
            writer.update(|val| *val += 1);

            doc.run_effects();

            assert_eq!(*value.lock(), 0);
        }
    }

    #[test]
    fn effect_signal_no_duplicate() {
        let rt = Runtime::new();
        let doc = Document::new(rt);
        let cx = doc.root_scope();

        let mut tree = LayoutTree::new();
        let mut events = Events::new();

        let cx = cx.push(create_node());

        let (reader, _) = create_signal(&cx, ());

        create_effect(&cx, move || {
            let _ = reader.get();
            let _ = reader.get();
        });

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        {
            let inner = doc.runtime.inner.lock();
            assert_eq!(inner.effects.len(), 1);

            let entry = inner.signal_effects.values().nth(0).unwrap();
            assert_eq!(entry.len(), 1);
        }
    }

    #[test]
    fn effect_cleanup() {
        let rt = Runtime::new();
        let doc = Document::new(rt);
        let cx = doc.root_scope();

        let mut tree = LayoutTree::new();
        let mut events = Events::new();

        let cx = cx.push(create_node());

        create_effect(&cx, move || {});

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        {
            let inner = doc.inner.lock();
            assert_eq!(inner.effects.len(), 1);
        }

        cx.remove(cx.id().unwrap());

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        {
            let inner = doc.inner.lock();
            assert_eq!(inner.effects.len(), 0);
        }
    }

    #[test]
    fn effect_cleanup_with_single_signal() {
        let rt = Runtime::new();
        let doc = Document::new(rt);
        let cx = doc.root_scope();

        let mut tree = LayoutTree::new();
        let mut events = Events::new();

        let cx = cx.push(create_node());

        let (reader, _) = create_signal(&cx, ());

        create_effect(&cx, move || {
            let _ = reader.get();
        });

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        {
            let inner = doc.runtime.inner.lock();
            assert_eq!(inner.signal_effects.len(), 1);

            let entry = inner.signal_effects.values().nth(0).unwrap();
            assert_eq!(entry.len(), 1);
        }

        cx.remove(cx.id().unwrap());

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        {
            let inner = doc.runtime.inner.lock();
            assert_eq!(inner.signal_effects.len(), 0);
        }
    }

    #[test]
    fn effect_cleanup_with_shared_signal() {
        let rt = Runtime::new();
        let doc = Document::new(rt);
        let cx = doc.root_scope();

        let mut tree = LayoutTree::new();
        let mut events = Events::new();

        let cx = cx.push(create_node());

        let (reader, _) = create_signal(&cx, ());

        for _ in 0..2 {
            let reader = reader.clone();

            create_effect(&cx, move || {
                let _ = reader.get();
            });
        }

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        {
            let inner = doc.runtime.inner.lock();
            assert_eq!(inner.signal_effects.len(), 1);

            let entry = inner.signal_effects.values().nth(0).unwrap();
            assert_eq!(entry.len(), 2);
        }

        cx.remove(cx.id().unwrap());

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        {
            let inner = doc.runtime.inner.lock();
            assert_eq!(inner.signal_effects.len(), 0);
        }
    }
}
