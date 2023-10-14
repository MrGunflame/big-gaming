use game_common::components::inventory::InventorySlotId;
use game_common::entity::EntityId;
use game_common::events::Event;
use game_common::world::world::WorldViewMut;
use game_common::world::CellId;
use wasmtime::{Engine, Instance, Linker, Module, Store};

use crate::effect::Effects;
use crate::events::{Events, OnAction, OnCellLoad, OnCellUnload, OnCollision, OnEquip, OnUnequip};

pub struct ScriptInstance<'world, 'view> {
    store: Store<State<'world, 'view>>,
    inner: Instance,
    events: Events,
}

impl<'world, 'view> ScriptInstance<'world, 'view> {
    pub fn new(
        engine: &Engine,
        module: &Module,
        events: Events,
        world: &'view mut WorldViewMut<'world>,
        physics_pipeline: &'view game_physics::Pipeline,
        effects: &'view mut Effects,
    ) -> Self {
        let mut store = Store::new(
            engine,
            State {
                world,
                physics_pipeline,
                effects,
            },
        );

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
            Event::Collision(event) => self.on_collision(event.entity, event.other),
            Event::Equip(event) => self.on_equip(event.item, event.entity),
            Event::Unequip(event) => self.on_unequip(event.item, event.entity),
            Event::CellLoad(event) => self.on_cell_load(event.cell),
            Event::CellUnload(event) => self.on_cell_unload(event.cell),
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

    pub fn on_equip(&mut self, item: InventorySlotId, entity: EntityId) -> wasmtime::Result<()> {
        let func: OnEquip = self.inner.get_typed_func(&mut self.store, "on_equip")?;
        func.call(&mut self.store, (item.into_raw(), entity.into_raw()))
    }

    pub fn on_unequip(&mut self, item: InventorySlotId, entity: EntityId) -> wasmtime::Result<()> {
        let func: OnUnequip = self.inner.get_typed_func(&mut self.store, "on_unequip")?;
        func.call(&mut self.store, (item.into_raw(), entity.into_raw()))
    }

    pub fn on_cell_load(&mut self, id: CellId) -> wasmtime::Result<()> {
        let func: OnCellLoad = self.inner.get_typed_func(&mut self.store, "on_cell_load")?;
        func.call(&mut self.store, id.as_parts())
    }

    pub fn on_cell_unload(&mut self, id: CellId) -> wasmtime::Result<()> {
        let func: OnCellUnload = self
            .inner
            .get_typed_func(&mut self.store, "on_cell_unload")?;
        func.call(&mut self.store, id.as_parts())
    }
}

pub struct State<'world, 'view> {
    pub world: &'view mut WorldViewMut<'world>,
    pub physics_pipeline: &'view game_physics::Pipeline,
    pub effects: &'view mut Effects,
}
