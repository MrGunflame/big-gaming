use std::sync::Arc;

use parking_lot::Mutex;
use slotmap::DefaultKey;

use crate::reactive::effect::EffectId;
use crate::reactive::ACTIVE_EFFECT;

use super::{NodeId, Scope};

pub fn create_signal<T>(cx: &Scope, value: T) -> (ReadSignal<T>, WriteSignal<T>)
where
    T: Send + Sync + 'static,
{
    tracing::trace!("creating reactive signal for node {:?}", cx.id);

    let signal = Signal { effects: vec![] };

    let mut doc = cx.document.inner.lock();

    let mut rt = cx.document.runtime.inner.lock();

    let id = rt.signals.insert(signal);
    doc.signal_targets.insert(id, cx.id.map(|x| x.0));
    rt.signal_effects.insert(id, vec![]);

    let value = Arc::new(Mutex::new(value));

    (
        ReadSignal {
            cx: cx.clone(),
            id: SignalId(id),
            value: value.clone(),
        },
        WriteSignal {
            cx: cx.clone(),
            id: SignalId(id),
            value,
        },
    )
}

#[derive(Debug)]
pub struct ReadSignal<T>
where
    T: Send + Sync + 'static,
{
    cx: Scope,
    id: SignalId,
    value: Arc<Mutex<T>>,
}

impl<T> ReadSignal<T>
where
    T: Send + Sync + 'static,
{
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(T::clone)
    }

    pub fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        self.with_untracked(T::clone)
    }

    pub fn with<U, F>(&self, f: F) -> U
    where
        F: FnOnce(&T) -> U,
    {
        self.track();

        let cell = self.value.lock();
        f(&cell)
    }

    pub fn with_mut<U, F>(&self, f: F) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        self.track();

        let mut cell = self.value.lock();
        f(&mut cell)
    }

    pub fn with_untracked<U, F>(&self, f: F) -> U
    where
        F: FnOnce(&T) -> U,
    {
        let cell = self.value.lock();
        f(&cell)
    }

    pub fn track(&self) {
        tracing::trace!("Signal({:?})::read", self.id);

        ACTIVE_EFFECT.with(|cell| {
            let mut data = cell.borrow_mut();
            if data.first_run {
                data.stack.push(self.id);
            }
        });
    }
}

impl<T> Clone for ReadSignal<T>
where
    T: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            cx: self.cx.clone(),
            id: self.id,
            value: self.value.clone(),
        }
    }
}

#[derive(Debug)]
pub struct WriteSignal<T>
where
    T: Send + Sync + 'static,
{
    cx: Scope,
    id: SignalId,
    value: Arc<Mutex<T>>,
}

impl<T> WriteSignal<T>
where
    T: Send + Sync + 'static,
{
    pub fn set(&self, value: T) {
        self.update(|cell| *cell = value);
    }

    pub fn set_untracked(&self, value: T) {
        self.update_untracked(|cell| *cell = value);
    }

    pub fn update<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        tracing::trace!("Signal({:?})::write", self.id);

        let ret = {
            let mut cell = self.value.lock();
            f(&mut cell)
        };

        self.wake();

        ret
    }

    pub fn update_untracked<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        tracing::trace!("Signal({:?})::write_untracked", self.id);

        let mut cell = self.value.lock();
        f(&mut cell)
    }

    pub fn subscribe(&self) -> ReadSignal<T> {
        tracing::trace!("Signal({:?})::subscribe", self.id);

        ReadSignal {
            cx: self.cx.clone(),
            id: self.id,
            value: self.value.clone(),
        }
    }

    /// Manually mark the value as changed.
    pub fn wake(&self) {
        let mut rt = self.cx.document.runtime.inner.lock();

        let effects = rt.signal_effects.get(&self.id.0).unwrap().clone();

        tracing::trace!(
            "Queued Signal({:?}) effect observers: {:?}",
            self.id,
            effects
        );

        rt.effect_queue.extend(effects.iter().map(|e| EffectId(*e)));
    }

    pub fn with<U, F>(&self, f: F) -> U
    where
        F: FnOnce(&T) -> U,
    {
        let cell = self.value.lock();
        f(&cell)
    }

    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(T::clone)
    }
}

impl<T> Clone for WriteSignal<T>
where
    T: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            cx: self.cx.clone(),
            id: self.id,
            value: self.value.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Signal {
    pub(super) effects: Vec<NodeId>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SignalId(pub DefaultKey);

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use bevy_ecs::world::World;
    use parking_lot::Mutex;

    use crate::events::Events;
    use crate::reactive::{create_effect, Document, Runtime};
    use crate::render::layout::LayoutTree;

    use super::create_signal;

    #[test]
    fn signal_update() {
        let rt = Runtime::new();
        let doc = Document::new(rt);
        let cx = doc.root_scope();

        let (reader, writer) = create_signal(&cx, 0);

        assert_eq!(reader.get(), 0);

        writer.update(|val| *val += 1);

        assert_eq!(reader.get(), 1);
    }

    #[test]
    fn signal_called_across_documents() {
        let count = 2;

        let value = Arc::new(Mutex::new(0));

        let rt = Runtime::new();
        let docs: Vec<_> = (0..count).map(|_| Document::new(rt.clone())).collect();

        let (reader, writer) = create_signal(&docs[0].root_scope(), 0);

        for doc in &docs {
            let cx = doc.root_scope();

            let reader = reader.clone();
            let value = value.clone();

            create_effect(&cx, move |_| {
                let _ = reader.get();

                *value.lock() += 1;
            });
        }

        let mut tree = LayoutTree::new();
        let mut events = Events::new();
        let world = World::new();

        for doc in &docs {
            doc.run_effects(&world);
            doc.flush_node_queue(&mut tree, &mut events);
        }

        assert_eq!(*value.lock(), count);

        writer.wake();

        for doc in &docs {
            doc.run_effects(&world);
            doc.flush_node_queue(&mut tree, &mut events);
        }

        assert_eq!(*value.lock(), count * 2);
    }

    #[test]
    fn signal_moved_across_documents() {
        let value = Arc::new(Mutex::new(0));

        let rt = Runtime::new();
        let src = Document::new(rt.clone());
        let dst = Document::new(rt);

        let world = World::new();

        let (reader, writer) = create_signal(&src.root_scope(), 0);

        {
            let value = value.clone();
            create_effect(&dst.root_scope(), move |_| {
                let _ = reader.get();

                *value.lock() += 1;
            });
        }

        tracing::trace!("src");
        src.run_effects(&world);
        tracing::trace!("dst");
        dst.run_effects(&world);

        assert_eq!(*value.lock(), 1);

        writer.wake();

        tracing::trace!("src");
        src.run_effects(&world);
        tracing::trace!("dst");
        dst.run_effects(&world);

        assert_eq!(*value.lock(), 2);
    }
}
