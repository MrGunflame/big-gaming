use std::sync::Mutex;

use bevy_ecs::prelude::Component;
use slotmap::{DefaultKey, SlotMap};

use crate::render::layout::LayoutTree;

use self::effect::Effect;
use self::signal::Signal;

static RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

static CAPTURE_SIGNALS: Mutex<Vec<NodeId>> = Mutex::new(vec![]);

mod effect;
mod signal;

pub use effect::create_effect;
pub use signal::create_signal;

pub fn init_runtime() {
    let mut rt = RUNTIME.lock().unwrap();
    *rt = Some(Runtime::default());
}

/// Reactive runtime.
#[derive(Default)]
pub struct Runtime {
    signals: SlotMap<DefaultKey, Signal>,
    effects: SlotMap<DefaultKey, Effect>,
}

fn uptime_signal<T>(id: NodeId, f: impl FnOnce(&mut T))
where
    T: Send + Sync + 'static,
{
    let effects = with_runtime(|rt| {
        if let Some(signal) = rt.signals.get_mut(id.0) {
            f(signal.value.downcast_mut().unwrap());
            signal.effects.clone()
        } else {
            vec![]
        }
    });

    for effect in &effects {
        run_effect(*effect);
    }
}

impl Runtime {
    fn with_signal<T, U>(&self, id: NodeId, f: impl FnOnce(&T) -> U) -> U
    where
        T: Send + Sync + 'static,
    {
        if let Some(signal) = self.signals.get(id.0) {
            let mut stack = CAPTURE_SIGNALS.lock().unwrap();
            stack.push(id);
            drop(stack);

            f(signal.value.downcast_ref().unwrap())
        } else {
            panic!("no such nodeid: {:?}", id)
        }
    }
}

fn run_effect(id: NodeId) {
    let effect = with_runtime(|rt| rt.effects.get(id.0).unwrap().to_owned());
    // Drop the runtime so the effect can access signals.

    (effect.f)();

    let mut stack = CAPTURE_SIGNALS.lock().unwrap();
    let stack = &mut *stack;

    if effect.is_first_run {
        with_runtime(|rt| {
            let effect = rt.effects.get_mut(id.0).unwrap();
            effect.is_first_run = false;
            effect.signals.extend(std::mem::take(stack).into_iter());

            for signal in &effect.signals {
                rt.signals.get_mut(signal.0).unwrap().effects.push(id);
            }
        });
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(DefaultKey);

fn with_runtime<T>(f: impl FnOnce(&mut Runtime) -> T) -> T {
    let mut rt = RUNTIME.lock().unwrap();
    let mut rt = rt.as_mut().unwrap();
    f(&mut rt)
}

pub struct Scope {
    id: NodeId,
    parent: Option<NodeId>,
}

#[derive(Component)]
pub struct ReactiveRoot {
    pub(crate) f: Box<dyn Fn(Scope) + Send + Sync + 'static>,
    pub(crate) is_first_run: bool,
}

impl ReactiveRoot {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(Scope) + Send + Sync + 'static,
    {
        Self {
            f: Box::new(f),
            is_first_run: true,
        }
    }
}
