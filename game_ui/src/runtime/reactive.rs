use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use game_tracing::trace_span;
use parking_lot::Mutex;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SignalId(u64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EffectId(u64);

#[derive(Clone, Debug)]
pub struct ReactiveRuntime {
    inner: Arc<Mutex<ContextInner>>,
}

impl ReactiveRuntime {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ContextInner {
                next_signal_id: SignalId(0),
                next_effect_id: EffectId(0),
                signal_effects: HashMap::new(),
                effects: HashMap::new(),
                effects_scheduled: HashSet::new(),
            })),
        }
    }

    pub fn create_signal<T>(&self, value: T) -> (ReadSignal<T>, WriteSignal<T>) {
        let mut inner = self.inner.lock();

        let id = inner.next_signal_id;
        inner.next_signal_id.0 += 1;

        let inner = Arc::new(SignalInner {
            ctx: self.clone(),
            id,
            read_count: Mutex::new(1),
            write_count: Mutex::new(1),
            value: Mutex::new(value),
        });

        (
            ReadSignal {
                inner: inner.clone(),
            },
            WriteSignal { inner },
        )
    }

    pub fn register_effect<T>(&self, effect: T) -> EffectId
    where
        T: Effect,
    {
        let mut inner = self.inner.lock();
        let id = inner.next_effect_id;
        inner.next_effect_id.0 += 1;

        let mut ctx = NodeContext {
            add_signals: Vec::new(),
            remove_signals: Vec::new(),
        };

        effect.init(&mut ctx);

        for signal in &ctx.add_signals {
            inner.signal_effects.entry(*signal).or_default().push(id);
        }

        inner.effects.insert(
            id,
            Arc::new(Mutex::new(Subscriber {
                sources: ctx.add_signals.into_iter().collect(),
                effect: Box::new(effect),
            })),
        );

        id
    }

    pub fn register_and_schedule_effect<T>(&self, effect: T) -> EffectId
    where
        T: Effect,
    {
        let id = self.register_effect(effect);

        let mut inner = self.inner.lock();
        inner.effects_scheduled.insert(id);

        id
    }

    pub(crate) fn update(&self) {
        let _span = trace_span!("ReactiveRuntime::update").entered();

        let mut inner = self.inner.lock();

        let mut effects = Vec::new();
        for id in core::mem::take(&mut inner.effects_scheduled) {
            let effect = inner.effects.get(&id).unwrap();
            effects.push((id, effect.clone()));
        }

        drop(inner);

        for (id, subscriber) in effects {
            let mut subscriber = subscriber.lock();

            let mut ctx = NodeContext {
                add_signals: Vec::new(),
                remove_signals: Vec::new(),
            };

            subscriber.effect.run(&mut ctx);

            let mut inner = self.inner.lock();

            for signal in ctx.add_signals {
                if !subscriber.sources.insert(signal) {
                    continue;
                }

                let entry = inner.signal_effects.entry(signal).or_default();
                entry.push(id);
            }

            for signal in ctx.remove_signals {
                if !subscriber.sources.remove(&signal) {
                    continue;
                }

                if let Some(subscribers) = inner.signal_effects.get_mut(&signal) {
                    subscribers.retain(|sub_id| *sub_id != id);
                }
            }

            // If the subscriber has no more sources it can never be
            // called again and we can remove it.
            if subscriber.sources.is_empty() {
                inner.effects.remove(&id);
            }
        }
    }

    fn unregister_signal(&self, id: SignalId) {
        let mut inner = self.inner.lock();

        inner.signal_effects.remove(&id);
    }
}

#[derive(Debug)]
struct ContextInner {
    next_signal_id: SignalId,
    next_effect_id: EffectId,
    signal_effects: HashMap<SignalId, Vec<EffectId>>,
    effects: HashMap<EffectId, Arc<Mutex<Subscriber>>>,
    effects_scheduled: HashSet<EffectId>,
}

#[derive(Clone, Debug)]
pub struct NodeContext {
    add_signals: Vec<SignalId>,
    remove_signals: Vec<SignalId>,
}

impl NodeContext {
    /// Subscribes the current node to changes from the given [`SignalId`].
    pub fn subscribe(&mut self, id: SignalId) {
        self.add_signals.push(id);
    }

    /// Unregisters the current node from the given [`SignalId`].
    ///
    /// If all [`SignalId`]s are unregistered the node will never be called again and may be
    /// dropped.
    pub fn unregister(&mut self, id: SignalId) {
        self.add_signals.retain(|signal| *signal != id);
        self.remove_signals.push(id);
    }
}

struct Subscriber {
    /// List of sources that can trigger this effect.
    sources: HashSet<SignalId>,
    effect: Box<dyn Effect>,
}

impl Debug for Subscriber {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Subscriber")
            .field("sources", &self.sources)
            .finish_non_exhaustive()
    }
}

pub trait Effect: Send + 'static {
    fn init(&self, ctx: &mut NodeContext);

    fn run(&mut self, ctx: &mut NodeContext);
}

impl<F> Effect for F
where
    F: FnMut(&mut NodeContext) + Send + 'static,
{
    fn init(&self, _ctx: &mut NodeContext) {}

    fn run(&mut self, ctx: &mut NodeContext) {
        self(ctx);
    }
}

#[derive(Debug)]
pub struct ReadSignal<T> {
    inner: Arc<SignalInner<T>>,
}

impl<T> ReadSignal<T> {
    pub fn id(&self) -> SignalId {
        self.inner.id
    }

    pub fn with<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&T) -> U,
    {
        let value = self.inner.value.lock();
        f(&value)
    }

    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.inner.value.lock().clone()
    }
}

impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        self.inner.increment_read_count();
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Drop for ReadSignal<T> {
    fn drop(&mut self) {
        self.inner.decrement_read_count();
    }
}

#[derive(Debug)]
pub struct WriteSignal<T> {
    inner: Arc<SignalInner<T>>,
}

impl<T> WriteSignal<T> {
    pub fn id(&self) -> SignalId {
        self.inner.id
    }

    pub fn update<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        let mut value = self.inner.value.lock();
        f(&mut value)
    }

    pub fn set(&self, value: T) {
        *self.inner.value.lock() = value;
        self.wake();
    }

    fn wake(&self) {
        let inner = &mut *self.inner.ctx.inner.lock();
        if let Some(effects) = inner.signal_effects.get_mut(&self.inner.id) {
            inner.effects_scheduled.extend(effects.iter().copied());
        }
    }
}

impl<T> Clone for WriteSignal<T> {
    fn clone(&self) -> Self {
        self.inner.increment_write_count();
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Drop for WriteSignal<T> {
    fn drop(&mut self) {
        self.inner.decrement_write_count();
    }
}

#[derive(Debug)]
struct SignalInner<T> {
    ctx: ReactiveRuntime,
    id: SignalId,
    value: Mutex<T>,
    read_count: Mutex<usize>,
    write_count: Mutex<usize>,
}

impl<T> SignalInner<T> {
    fn increment_write_count(&self) {
        *self.write_count.lock() += 1;
    }

    fn decrement_write_count(&self) {
        let mut count = self.write_count.lock();
        *count -= 1;

        if *count != 0 {
            return;
        }

        self.ctx.unregister_signal(self.id);
    }

    fn increment_read_count(&self) {
        *self.read_count.lock() += 1;
    }

    fn decrement_read_count(&self) {
        let mut count = self.read_count.lock();
        *count -= 1;

        if *count != 0 {
            return;
        }

        self.ctx.unregister_signal(self.id);
    }
}
