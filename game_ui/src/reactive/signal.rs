use std::sync::Arc;

use parking_lot::Mutex;

use super::{NodeId, Scope};

pub fn create_signal<T>(cx: &Scope, value: T) -> (ReadSignal<T>, WriteSignal<T>)
where
    T: Send + Sync + 'static,
{
    tracing::trace!("creating reactive signal for node {:?}", cx.parent);

    let signal = Signal { effects: vec![] };

    let mut doc = cx.document.inner.lock();
    let id = doc.signals.insert(signal);

    if let Some(sid) = cx.id {
        doc.signal_targets.insert(id, sid.0);
    }

    let value = Arc::new(Mutex::new(value));

    (
        ReadSignal {
            cx: cx.clone(),
            id: NodeId(id),
            value: value.clone(),
        },
        WriteSignal {
            cx: cx.clone(),
            id: NodeId(id),
            value,
        },
    )
}

#[derive(Clone)]
pub struct ReadSignal<T>
where
    T: Send + Sync + 'static,
{
    cx: Scope,
    id: NodeId,
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

    pub fn with<U, F>(&self, f: F) -> U
    where
        F: FnOnce(&T) -> U,
    {
        tracing::trace!("Signal({:?})::read", self.id);

        let mut cell = self.value.lock();
        f(&cell)
    }
}

#[derive(Clone)]
pub struct WriteSignal<T>
where
    T: Send + Sync + 'static,
{
    cx: Scope,
    id: NodeId,
    value: Arc<Mutex<T>>,
}

impl<T> WriteSignal<T>
where
    T: Send + Sync + 'static,
{
    pub fn set(&self, value: T) {
        self.update(|cell| *cell = value);
    }

    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        tracing::trace!("Signal({:?})::write", self.id);

        {
            let mut cell = self.value.lock();
            f(&mut cell);
        }

        let mut doc = self.cx.document.inner.lock();
        doc.signal_queue.push(self.id.0);
    }
}

#[derive(Clone, Debug)]
pub(super) struct Signal {
    pub(super) effects: Vec<NodeId>,
}
