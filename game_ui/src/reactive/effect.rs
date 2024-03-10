use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use parking_lot::Mutex;
use slotmap::new_key_type;

use super::Scope;

impl Scope {
    pub fn create_effect<F>(&self, f: F)
    where
        F: FnMut() + Send + Sync + 'static,
    {
        let effect = Effect {
            f: Arc::new(Mutex::new(f)),
            first_run: true,
        };

        let mut doc = self.document.inner.lock();
        let mut rt = self.document.runtime.inner.lock();

        let id = rt.effects.insert(effect);

        doc.effects.insert(id);
        doc.effects_by_node.entry(self.id).or_default().push(id);

        // Queue for immediate execution.
        rt.queue.insert(id);
    }
}

#[derive(Clone)]
pub(super) struct Effect {
    pub f: Arc<Mutex<dyn FnMut() + Send + Sync + 'static>>,
    pub first_run: bool,
}

impl Debug for Effect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Effect").finish_non_exhaustive()
    }
}

new_key_type! {
    pub(super) struct EffectId;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::Mutex;

    use crate::events::Events;
    use crate::layout::LayoutTree;
    use crate::reactive::{Document, Runtime};

    use super::super::tests::create_node;

    #[test]
    fn effect_call_on_creation() {
        let rt = Runtime::new();
        let doc = Document::new(rt);
        let cx = doc.root_scope();

        let value = Arc::new(Mutex::new(0));

        {
            let value = value.clone();
            cx.create_effect(move || {
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

        let (reader, writer) = cx.create_signal(0);

        {
            let value = value.clone();
            cx.create_effect(move || {
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

        let (reader, writer) = cx.create_signal(0);

        {
            let value = value.clone();
            cx.create_effect(move || {
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

        let (reader, _) = cx.create_signal(());

        cx.create_effect(move || {
            let _ = reader.get();
            let _ = reader.get();
        });

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        {
            let inner = doc.runtime.inner.lock();
            assert_eq!(inner.effects.len(), 1);

            let entry = inner.subscribers.values().nth(0).unwrap();
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

        cx.create_effect(move || {});

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

        let (reader, _) = cx.create_signal(());

        cx.create_effect(move || {
            let _ = reader.get();
        });

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        {
            let inner = doc.runtime.inner.lock();
            assert_eq!(inner.subscribers.len(), 1);

            let entry = inner.subscribers.values().nth(0).unwrap();
            assert_eq!(entry.len(), 1);
        }

        cx.remove(cx.id().unwrap());

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        {
            let inner = doc.runtime.inner.lock();
            assert_eq!(inner.subscribers.len(), 0);
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

        let (reader, _) = cx.create_signal(());

        for _ in 0..2 {
            let reader = reader.clone();

            cx.create_effect(move || {
                let _ = reader.get();
            });
        }

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        {
            let inner = doc.runtime.inner.lock();
            assert_eq!(inner.subscribers.len(), 1);

            let entry = inner.subscribers.values().nth(0).unwrap();
            assert_eq!(entry.len(), 2);
        }

        cx.remove(cx.id().unwrap());

        doc.run_effects();
        doc.flush_node_queue(&mut tree, &mut events);

        {
            let inner = doc.runtime.inner.lock();
            assert_eq!(inner.subscribers.len(), 0);
        }
    }
}
