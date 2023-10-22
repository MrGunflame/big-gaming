use std::marker::PhantomData;

use game_common::components::components::Component;
use game_common::components::inventory::InventorySlotId;
use game_common::entity::EntityId;
use game_common::events::Event;
use game_common::record::RecordReference;
use game_common::world::entity::Entity;
use game_common::world::CellId;
use glam::{Quat, Vec3};
use wasmtime::{Engine, Instance, Linker, Module, Store};

use crate::effect::{Effect, Effects};
use crate::events::{Events, OnAction, OnCellLoad, OnCellUnload, OnCollision, OnEquip, OnUnequip};
use crate::WorldProvider;

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
        world: &'view dyn WorldProvider,
        physics_pipeline: &'view game_physics::Pipeline,
        effects: &'view mut Effects,
    ) -> Self {
        let mut store = Store::new(
            engine,
            State {
                world,
                physics_pipeline,
                effects,
                _stub: PhantomData,
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
    pub world: &'view dyn WorldProvider,
    pub physics_pipeline: &'view game_physics::Pipeline,
    pub effects: &'view mut Effects,
    _stub: PhantomData<&'world ()>,
}

impl State<'_, '_> {
    pub fn spawn(&mut self, entity: Entity) -> EntityId {
        self.effects.push(Effect::EntitySpawn(entity));
        todo!()
    }

    pub fn get(&self, id: EntityId) -> Option<Entity> {
        self.reconstruct_entity(id)
    }

    pub fn despawn(&mut self, id: EntityId) -> Option<Entity> {
        if let Some(entity) = self.reconstruct_entity(id) {
            self.effects.push(Effect::EntityDespawn(id));
            Some(entity)
        } else {
            None
        }
    }

    pub fn set_translation(&mut self, id: EntityId, translation: Vec3) -> bool {
        if self.reconstruct_entity(id).is_some() {
            self.effects.push(Effect::EntityTranslate(id, translation));
            true
        } else {
            false
        }
    }

    pub fn set_rotation(&mut self, id: EntityId, rotation: Quat) -> bool {
        if self.reconstruct_entity(id).is_some() {
            self.effects.push(Effect::EntityRotate(id, rotation));
            true
        } else {
            false
        }
    }

    pub fn get_component(
        &self,
        entity_id: EntityId,
        component: RecordReference,
    ) -> Option<&Component> {
        let entity = self.reconstruct_entity(entity_id)?;
        entity.components.get(component)
    }

    pub fn insert_component(
        &mut self,
        entity_id: EntityId,
        id: RecordReference,
        component: Component,
    ) {
        self.effects.push(Effect::EntityComponentInsert(
            entity_id,
            id,
            component.bytes,
        ));
    }

    pub fn remove_component(&mut self, entity_id: EntityId, id: RecordReference) -> bool {
        if let Some(ent) = self.reconstruct_entity(entity_id) {
            if ent.components.get(id).is_some() {
                self.effects
                    .push(Effect::EntityComponentRemove(entity_id, id));
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn reconstruct_entity(&self, id: EntityId) -> Option<Entity> {
        let mut entity = self.world.get(id).cloned();

        for effect in self.effects.iter() {
            match effect {
                Effect::EntitySpawn(e) if e.id == id => {
                    entity = Some(e.clone());
                }
                Effect::EntityDespawn(eid) if *eid == id => {
                    entity = None;
                }
                Effect::EntityTranslate(eid, translation) if *eid == id => {
                    entity.as_mut().unwrap().transform.translation = *translation;
                }
                Effect::EntityRotate(eid, rotation) if *eid == id => {
                    entity.as_mut().unwrap().transform.rotation = *rotation;
                }
                Effect::EntityComponentInsert(eid, cid, bytes) if *eid == id => {
                    entity.as_mut().unwrap().components.insert(
                        *cid,
                        Component {
                            bytes: bytes.to_vec(),
                        },
                    );
                }
                Effect::EntityComponentRemove(eid, cid) if *eid == id => {
                    entity.as_mut().unwrap().components.remove(*cid);
                }
            }
        }

        entity
    }
}
