use std::panic::Location;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::reactive::ACTIVE_EFFECT;

use super::Scope;

impl Scope {
    pub fn create_signal<T>(&self, value: T) -> (ReadSignal<T>, WriteSignal<T>)
    where
        T: Send + Sync + 'static,
    {
        let value = Arc::new(Mutex::new(value));

        let mut rt = self.document.runtime.inner.lock();

        let id = SignalId(rt.next_signal_id);
        rt.next_signal_id += 1;

        (
            ReadSignal {
                cx: self.clone(),
                id,
                value: value.clone(),
            },
            WriteSignal {
                cx: self.clone(),
                id,
                value,
            },
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct SignalId(u64);

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
        tracing::trace!("waking Signal({:?})", self.id);

        let mut rt = self.cx.document.runtime.inner.lock();

        let Some(effects) = rt.subscribers.get(&self.id).cloned() else {
            return;
        };

        tracing::trace!(
            "Queued Signal({:?}) effect observers: {:?}",
            self.id,
            effects
        );

        rt.queue.extend(effects);
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
    // Location exists purely for the derived debug impl.
    #[cfg(debug_assertions)]
    #[allow(unused)]
    pub location: &'static Location<'static>,
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use parking_lot::Mutex;

    use crate::events::Events;
    use crate::layout::LayoutTree;
    use crate::reactive::{Document, Runtime};

    #[test]
    fn signal_update() {
        let rt = Runtime::new();
        let doc = Document::new(rt);
        let cx = doc.root_scope();

        let (reader, writer) = cx.create_signal(0);

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

        let (reader, writer) = docs[0].root_scope().create_signal(0);

        for doc in &docs {
            let cx = doc.root_scope();

            let reader = reader.clone();
            let value = value.clone();

            cx.create_effect(move || {
                let _ = reader.get();

                *value.lock() += 1;
            });
        }

        let mut tree = LayoutTree::new();
        let mut events = Events::new();

        for doc in &docs {
            doc.run_effects();
            doc.flush_node_queue(&mut tree, &mut events);
        }

        assert_eq!(*value.lock(), count);

        writer.wake();

        for doc in &docs {
            doc.run_effects();
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

        let (reader, writer) = src.root_scope().create_signal(0);

        {
            let value = value.clone();
            dst.root_scope().create_effect(move || {
                let _ = reader.get();

                *value.lock() += 1;
            });
        }

        src.run_effects();
        dst.run_effects();

        assert_eq!(*value.lock(), 1);

        writer.wake();

        src.run_effects();
        dst.run_effects();

        assert_eq!(*value.lock(), 2);
    }
}
