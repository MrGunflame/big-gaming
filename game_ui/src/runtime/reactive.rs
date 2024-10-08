use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;
use std::sync::Arc;

use parking_lot::Mutex;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SignalId(u64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct EffectId(u64);

#[derive(Clone, Debug)]
pub struct ReactiveContext {
    inner: Arc<Mutex<ContextInner>>,
}

impl ReactiveContext {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ContextInner {
                next_signal_id: SignalId(0),
                next_effect_id: EffectId(0),
                signal_effects: HashMap::new(),
                effects: HashMap::new(),
                signals_updated: HashSet::new(),
            })),
        }
    }

    pub fn create_signal<T>(&self, value: T) -> (ReadSignal<T>, WriteSignal<T>) {
        let mut inner = self.inner.lock();

        let id = inner.next_signal_id;
        inner.next_signal_id.0 += 1;

        let inner = Rc::new(SignalInner {
            ctx: self.clone(),
            id,
            read_count: Cell::new(1),
            write_count: Cell::new(1),
            value: RefCell::new(value),
        });

        (
            ReadSignal {
                inner: inner.clone(),
            },
            WriteSignal { inner },
        )
    }

    pub fn register_effect<F>(&self, signals: &[SignalId], f: F)
    where
        F: FnMut() + 'static,
    {
        let mut inner = self.inner.lock();
        let id = inner.next_effect_id;
        inner.next_effect_id.0 += 1;

        inner.effects.insert(
            id,
            Effect {
                signal_count: signals.len(),
                f: Rc::new(RefCell::new(f)),
            },
        );

        for signal in signals {
            inner.signal_effects.insert(*signal, id);
        }
    }

    fn unregister_signal(&self, id: SignalId) {
        let mut inner = self.inner.lock();

        let effect_id = inner.signal_effects.remove(&id).unwrap();
        let effect = inner.effects.get_mut(&effect_id).unwrap();
        effect.signal_count -= 1;
        if effect.signal_count == 0 {
            inner.effects.remove(&effect_id);
        }
    }
}

#[derive(Debug)]
struct ContextInner {
    next_signal_id: SignalId,
    next_effect_id: EffectId,
    signal_effects: HashMap<SignalId, EffectId>,
    effects: HashMap<EffectId, Effect>,
    signals_updated: HashSet<SignalId>,
}

struct Effect {
    signal_count: usize,
    f: Rc<RefCell<dyn FnMut()>>,
}

impl Debug for Effect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Effect")
            .field("signal_count", &self.signal_count)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct ReadSignal<T> {
    inner: Rc<SignalInner<T>>,
}

impl<T> ReadSignal<T> {
    pub fn id(&self) -> SignalId {
        self.inner.id
    }

    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.inner.value.borrow().clone()
    }
}

#[derive(Debug)]
pub struct WriteSignal<T> {
    inner: Rc<SignalInner<T>>,
}

impl<T> WriteSignal<T> {
    pub fn id(&self) -> SignalId {
        self.inner.id
    }

    pub fn set(&self, value: T) {
        *self.inner.value.borrow_mut() = value;
        self.wake();
    }

    fn wake(&self) {
        let mut inner = self.inner.ctx.inner.lock();
        inner.signals_updated.insert(self.inner.id);
    }
}

impl<T> Clone for WriteSignal<T> {
    fn clone(&self) -> Self {
        self.inner.increment_read_count();
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
    ctx: ReactiveContext,
    id: SignalId,
    value: RefCell<T>,
    read_count: Cell<usize>,
    write_count: Cell<usize>,
}

impl<T> SignalInner<T> {
    fn increment_write_count(&self) {
        let count = self.write_count.get();
        self.write_count.set(count + 1);
    }

    fn decrement_write_count(&self) {
        let count = self.write_count.get();
        self.write_count.set(count - 1);

        if count != 1 {
            return;
        }

        self.ctx.unregister_signal(self.id);
    }

    fn increment_read_count(&self) {
        let count = self.read_count.get();
        self.read_count.set(count + 1);
    }

    fn decrement_read_count(&self) {
        let count = self.read_count.get();
        self.read_count.set(count - 1);

        if count != 1 {
            return;
        }

        self.ctx.unregister_signal(self.id);
    }
}
