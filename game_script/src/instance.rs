use game_common::entity::EntityId;
use game_common::events::Event;
use game_common::world::world::WorldViewMut;
use wasmtime::{Engine, Instance, Linker, Module, Store};

use crate::events::{Events, OnAction, OnCollision};
use crate::queue::CommandQueue;

pub struct ScriptInstance<'world> {
    store: Store<State<'world>>,
    inner: Instance,
    events: Events,
}

impl<'world> ScriptInstance<'world> {
    pub fn new(
        engine: &Engine,
        module: &Module,
        events: Events,
        world: WorldViewMut<'world>,
        queue: &'world mut CommandQueue,
    ) -> Self {
        let mut store = Store::new(engine, State { queue, world });

        let mut linker = Linker::<State>::new(&engine);

        crate::builtin::register_host_fns(&mut linker);

        let instance = linker.instantiate(&mut store, &module).unwrap();

        Self {
            store,
            inner: instance,
            events,
        }
    }

    pub fn run(&mut self, event: &Event) -> wasmtime::Result<()> {
        tracing::info!("exec {:?}", event);

        match event {
            Event::Action(event) => self.on_action(event.entity, event.invoker),
            Event::Collision { entity, other } => self.on_collision(*entity, *other),
        }
    }

    pub fn on_action(&mut self, entity: EntityId, invoker: EntityId) -> wasmtime::Result<()> {
        let func: OnAction = self.inner.get_typed_func(&mut self.store, "on_action")?;
        func.call(&mut self.store, (entity.into_raw(), invoker.into_raw()))
    }

    pub fn on_collision(&mut self, entity: EntityId, other: EntityId) -> wasmtime::Result<()> {
        let func: OnCollision = self.inner.get_typed_func(&mut self.store, "on_collision")?;
        func.call(&mut self.store, (entity.into_raw(), other.into_raw()))
    }
}

pub struct State<'world> {
    pub queue: &'world mut CommandQueue,
    pub world: WorldViewMut<'world>,
}