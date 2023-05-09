use std::any::Any;
use std::marker::PhantomData;

use super::{NodeId, Scope};

pub fn create_signal<T>(value: T) -> (ReadSignal<T>, WriteSignal<T>)
where
    T: Send + Sync + 'static,
{
    let signal = Signal {
        value: Box::new(value),
        effects: vec![],
    };

    let id = super::with_runtime(|rt| rt.signals.insert(signal));

    (
        ReadSignal {
            id: NodeId(id),
            _marker: PhantomData,
        },
        WriteSignal {
            id: NodeId(id),
            _marker: PhantomData,
        },
    )
}

#[derive(Clone, Debug)]
pub struct ReadSignal<T>
where
    T: Send + Sync + 'static,
{
    id: NodeId,
    _marker: PhantomData<T>,
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
        super::with_runtime(|rt| rt.with_signal(self.id, f))
    }
}

#[derive(Clone, Debug)]
pub struct WriteSignal<T>
where
    T: Send + Sync + 'static,
{
    id: NodeId,
    _marker: PhantomData<T>,
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
        super::with_runtime(|rt| {
            rt.uptime_signal(self.id, f);
        });
    }
}

pub(super) struct Signal {
    pub(super) value: Box<dyn Any + Send + Sync + 'static>,
    pub(super) effects: Vec<NodeId>,
}
