use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use game_tracing::trace_span;
use parking_lot::Mutex;

/// A unique identifier for a signal.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SignalId(u64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EffectId(u64);

/// A runtime for a reactive context.
#[derive(Clone, Debug)]
pub struct ReactiveRuntime {
    inner: Arc<Mutex<ContextInner>>,
}

impl ReactiveRuntime {
    /// Creates a new `ReactiveRuntime`.
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

    /// Creates a new [`ReadSignal`]/[`WriteSignal`] pair with the given initial `value`.
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

    /// Registers a new [`Effect`].
    ///
    /// Note that the effect is not immediately scheduled which means that if [`init`] does not
    /// register sources the effect will never trigger and may be dropped immediately.
    ///
    /// See [`register_and_schedule_effect`] for a version that schedules the effect immediately,
    /// even if [`init`] does not register any sources.
    ///
    /// [`init`]: Effect::init
    /// [`register_and_schedule_effect`]: Self::register_and_schedule_effect
    pub fn register_effect<T>(&self, effect: T) -> EffectId
    where
        T: Effect,
    {
        let _span = trace_span!("ReactiveRuntime::register_effect").entered();

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

    /// Registers a new [`Effect`] and immediately schedules for execution.
    pub fn register_and_schedule_effect<T>(&self, effect: T) -> EffectId
    where
        T: Effect,
    {
        let _span = trace_span!("ReactiveRuntime::register_and_schedule_effect").entered();

        let id = self.register_effect(effect);

        let mut inner = self.inner.lock();
        inner.effects_scheduled.insert(id);

        id
    }

    /// Runs an update cycle on the `ReactiveRuntime`.
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

/// A effect is a observer that is called whenever a signal changes.
pub trait Effect: Send + 'static {
    /// Initializes the state of this `Effect`.
    ///
    /// This function is called exactly once when the `Effect` is first registered.
    fn init(&self, ctx: &mut NodeContext);

    /// Executes the `Effect` once.
    ///
    /// This function is called whenever the subscriber wakes up the `Effect`.
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

/// The read handle to a signal.
#[derive(Debug)]
pub struct ReadSignal<T>
where
    T: ?Sized,
{
    inner: Arc<SignalInner<T>>,
}

impl<T> ReadSignal<T>
where
    T: ?Sized,
{
    /// Returns the [`SignalId`] of the underyling signal.
    pub fn id(&self) -> SignalId {
        self.inner.id
    }

    /// Runs the given closure `F` on the underlying value.
    ///
    /// Note that `with` should not be nested with operations on the same underlying signal. The
    /// effects may include deadlocks or panics:
    /// ```no_run
    /// # fn main(rt: &ReactiveRutime) {
    /// let (value, set_value) = rt.create_signal(0);
    /// value.with(|| {
    ///     set_value.set(1); // <-- Don't do this
    /// });
    /// # }
    /// ```
    ///
    /// [`get`] does not have this potential for bugs and should be preferred if possible.
    ///
    /// [`get`]: Self::get
    pub fn with<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&T) -> U,
    {
        let value = self.inner.value.lock();
        f(&value)
    }

    /// Returns the underlying value.
    pub fn get(&self) -> T
    where
        T: Sized + Clone,
    {
        self.inner.value.lock().clone()
    }
}

impl<T> Clone for ReadSignal<T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        self.inner.increment_read_count();
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Drop for ReadSignal<T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        self.inner.decrement_read_count();
    }
}

/// A write handle to a signal.
#[derive(Debug)]
pub struct WriteSignal<T>
where
    T: ?Sized,
{
    inner: Arc<SignalInner<T>>,
}

impl<T> WriteSignal<T>
where
    T: ?Sized,
{
    /// Returns the [`SignalId`] of the underlying signal.
    pub fn id(&self) -> SignalId {
        self.inner.id
    }

    /// Updates the underlying value with the given closure.
    pub fn update<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        let mut value = self.inner.value.lock();
        f(&mut value)
    }

    /// Sets the value of the underlying signal to `value`.
    pub fn set(&self, value: T)
    where
        T: Sized,
    {
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

impl<T> Clone for WriteSignal<T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        self.inner.increment_write_count();
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Drop for WriteSignal<T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        self.inner.decrement_write_count();
    }
}

#[derive(Debug)]
struct SignalInner<T>
where
    T: ?Sized,
{
    ctx: ReactiveRuntime,
    id: SignalId,
    read_count: Mutex<usize>,
    write_count: Mutex<usize>,
    value: Mutex<T>,
}

impl<T> SignalInner<T>
where
    T: ?Sized,
{
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
