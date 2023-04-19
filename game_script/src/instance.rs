use std::mem::MaybeUninit;

use bytemuck::{AnyBitPattern, NoUninit};
use game_common::components::components::{Component, RecordReference};
use game_common::entity::EntityId;
use game_common::events::Event;
use game_common::world::entity::EntityBody;
use game_common::world::world::{WorldState, WorldViewMut};
use game_wasm::log::Level;
use game_wasm::raw;
use game_wasm::raw::world::{Entity, Item};
use wasmtime::{
    AsContextMut, Caller, Engine, Func, FuncType, Instance, Linker, Module, Store, TypedFunc, Val,
};

use crate::events::{Events, OnAction, OnCollision};

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
    ) -> Self {
        let mut store = Store::new(engine, State { world });

        let mut linker = Linker::<State>::new(&engine);

        crate::builtin::register_host_fns(&mut linker);

        let instance = linker.instantiate(&mut store, &module).unwrap();

        Self {
            store,
            inner: instance,
            events,
        }
    }

    pub fn run(&mut self, event: &Event) {
        tracing::info!("exec {:?}", event);

        match event {
            Event::Action(event) => self.on_action(event.entity, event.invoker),
            Event::Collision { entity, other } => self.on_collision(*entity, *other),
        }
    }

    pub fn on_action(&mut self, entity: EntityId, invoker: EntityId) {
        let func: OnAction = self
            .inner
            .get_typed_func(&mut self.store, "on_action")
            .unwrap();

        func.call(&mut self.store, (entity.into_raw(), invoker.into_raw()))
            .unwrap();
    }

    pub fn on_collision(&mut self, entity: EntityId, other: EntityId) {
        let func: OnCollision = self
            .inner
            .get_typed_func(&mut self.store, "on_collision")
            .unwrap();

        func.call(&mut self.store, (entity.into_raw(), other.into_raw()))
            .unwrap();
    }
}

pub struct State<'world> {
    pub world: WorldViewMut<'world>,
}
