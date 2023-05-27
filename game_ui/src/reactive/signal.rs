use std::sync::Arc;

use parking_lot::Mutex;
use slotmap::DefaultKey;

use crate::reactive::effect::EffectId;

use super::{NodeId, Scope};

pub fn create_signal<T>(cx: &Scope, value: T) -> (ReadSignal<T>, WriteSignal<T>)
where
    T: Send + Sync + 'static,
{
    tracing::trace!("creating reactive signal for node {:?}", cx.parent);

    let signal = Signal { effects: vec![] };

    let mut doc = cx.document.inner.lock();
    let id = doc.signals.insert(signal);
    doc.signal_targets.insert(id, cx.id.map(|x| x.0));
    doc.signal_effects.insert(id, vec![]);

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

#[derive(Clone)]
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
        tracing::trace!("Signal({:?})::read", self.id);

        let mut stack = self.cx.document.signal_stack.lock();
        stack.push(self.id);
        drop(stack);

        let cell = self.value.lock();
        f(&cell)
    }

    pub fn with_mut<U, F>(&self, f: F) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        tracing::trace!("Signal({:?})::read", self.id);

        let mut stack = self.cx.document.signal_stack.lock();
        stack.push(self.id);
        drop(stack);

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
}

#[derive(Debug, Clone)]
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
        let effects = doc.signal_effects.get(&self.id.0).unwrap().clone();

        tracing::trace!(
            "Queued Signal({:?}) effect observers: {:?}",
            self.id,
            effects
        );

        doc.effect_queue
            .extend(effects.iter().map(|e| EffectId(*e)));
    }

    pub fn subscribe(&self) -> ReadSignal<T> {
        tracing::trace!("Signal({:?})::subscribe", self.id);

        ReadSignal {
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
