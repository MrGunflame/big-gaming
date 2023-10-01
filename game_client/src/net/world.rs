use std::collections::VecDeque;

use game_common::components::actions::{ActionId, Actions};
use game_common::components::components::{self, Components};
use game_common::components::inventory::Inventory;
use game_common::components::items::Item;
use game_common::entity::EntityId;
use game_common::events::{ActionEvent, Event};
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::{Entity, EntityBody};
use game_common::world::snapshot::{EntityChange, InventoryItemAdd};
use game_common::world::world::{WorldState, WorldViewRef};
use game_core::modules::Modules;
use game_net::message::DataMessageBody;
use game_script::executor::ScriptExecutor;
use game_script::Context;
use glam::{Quat, Vec3};

use super::ServerConnection;

pub fn apply_world_delta<I>(
    conn: &mut ServerConnection<I>,
    cmd_buffer: &mut CommandBuffer,
    executor: &ScriptExecutor,
) {
    let cf = conn.control_frame();

    // Don't start rendering if the initial interpoation window is not
    // yet filled.
    let Some(render_cf) = cf.render else {
        return;
    };

    debug_assert!(conn.world.len() >= 2);

    // Called while we're still on the same frame, we won't do anything.
    // FIXME: This check might better be moved to a different place, like
    // being only called once the tick is stepped forward for example.
    if conn.last_render_frame == Some(render_cf) {
        return;
    }

    tracing::trace!(
        "Applying CF {:?}",
        conn.world.at(0).unwrap().control_frame()
    );

    // The first time a frame is being produced, i.e. `last_render_frame` is
    // `None` we must produce a "diff" consisting of the entire world state.
    let (delta, should_pop) = if conn.last_render_frame.is_none() {
        let view = conn.world.at(0).unwrap();
        (create_initial_diff(view), false)
    } else {
        let prev = conn.world.at(0).unwrap();
        let next = conn.world.at(1).unwrap();
        (create_snapshot_diff(prev, next), true)
    };

    // Since events are received in batches, and commands are not applied until
    // the system is done, we buffer all created entities so we can modify them
    // in place within the same batch before they are spawned into the world.
    let mut buffer = Buffer::new();

    for event in delta {
        handle_event(event.clone(), &mut buffer, conn, cmd_buffer, render_cf);
    }

    for msg in conn.input_buffer.iter() {
        match msg.body {
            DataMessageBody::EntityTranslate(msg) => {
                let id = conn.server_entities.get(msg.entity).unwrap();
                cmd_buffer.push(Command::Translate {
                    entity: id,
                    start: render_cf,
                    end: render_cf + 1,
                    dst: msg.translation,
                });
            }
            DataMessageBody::EntityRotate(msg) => {
                let id = conn.server_entities.get(msg.entity).unwrap();
                cmd_buffer.push(Command::Rotate {
                    entity: id,
                    start: render_cf,
                    end: render_cf + 1,
                    dst: msg.rotation,
                });
            }
            DataMessageBody::EntityAction(msg) => {
                // We don't directly handle actions here.
                // Actions are queued and handled at a later stage.
            }
            _ => {
                // Should never be sent from the client.
                if cfg!(debug_assertions) {
                    unreachable!();
                }
            }
        }
    }

    for entity in buffer.entities {
        conn.trace.spawn(render_cf, entity.entity.clone());

        cmd_buffer.push(Command::Spawn(entity));
    }

    if should_pop {
        conn.world.pop();
    }

    // We need to replicate the world snapshot as the client
    // predicted it.
    {
        let mut snapshot = conn.world.front().unwrap().snapshot().clone();

        for msg in conn.input_buffer.iter() {
            match msg.body {
                DataMessageBody::EntityTranslate(msg) => {
                    let id = conn.server_entities.get(msg.entity).unwrap();
                    let entity = snapshot.entities.get_mut(id).unwrap();
                    entity.transform.translation = msg.translation;
                }
                DataMessageBody::EntityRotate(msg) => {
                    let id = conn.server_entities.get(msg.entity).unwrap();
                    let entity = snapshot.entities.get_mut(id).unwrap();
                    entity.transform.rotation = msg.rotation;
                }
                DataMessageBody::EntityAction(msg) => {
                    let id = conn.server_entities.get(msg.entity).unwrap();
                    conn.event_queue.push(Event::Action(ActionEvent {
                        entity: id,
                        invoker: id,
                        action: msg.action,
                    }));
                }
                _ => {
                    // Should never be sent from the client.
                    if cfg!(debug_assertions) {
                        unreachable!();
                    }
                }
            }
        }

        conn.current_state = Some(snapshot);
    }

    conn.input_buffer.clear(render_cf);

    conn.last_render_frame = Some(render_cf);
}

fn handle_event<I>(
    event: EntityChange,
    buffer: &mut Buffer,
    conn: &mut ServerConnection<I>,
    cmd_buffer: &mut CommandBuffer,
    render_cf: ControlFrame,
) {
    // Frame that is being interpolated from.
    let view = conn.world.at(0).unwrap();

    tracing::trace!(
        concat!("handle ", stringify!(WorldState), " event: {:?}"),
        event
    );

    // Create and Destroy require special treatment.
    if !matches!(
        event,
        EntityChange::Create { entity: _ } | EntityChange::Destroy { id: _ }
    ) {
        let entity_id = event.entity();
        if let Some(entity) = buffer.get_mut(entity_id) {
            match event {
                EntityChange::Create { entity: _ } => {}
                EntityChange::Destroy { id: _ } => {}
                EntityChange::Translate { id: _, translation } => {
                    entity.entity.transform.translation = translation;
                }
                EntityChange::Rotate { id: _, rotation } => {
                    entity.entity.transform.rotation = rotation;
                }
                EntityChange::Health { id, health } => match &mut entity.entity.body {
                    EntityBody::Actor(actor) => actor.health = health,
                    _ => {
                        tracing::warn!("tried to apply health to a non-actor entity: {:?}", id);
                    }
                },
                EntityChange::CreateHost { id: _ } => entity.host = true,
                EntityChange::DestroyHost { id: _ } => entity.host = false,
                EntityChange::InventoryItemAdd(event) => {
                    //add_inventory_item(&mut entity.inventory, modules, event);
                }
                EntityChange::InventoryItemRemove(event) => {
                    entity.inventory.remove(event.id);
                }
                EntityChange::InventoryDestroy(event) => {
                    entity.inventory.clear();
                }
                EntityChange::CreateStreamingSource { id, source } => {}
                EntityChange::RemoveStreamingSource { id } => {}
            }

            return;
        }
    }

    match event {
        EntityChange::Create { entity } => {
            tracing::debug!("spawning entity {:?}", entity);

            buffer.push(entity);
        }
        EntityChange::Destroy { id } => {
            if !buffer.remove(id) {
                cmd_buffer.push(Command::Despawn(id));
            }

            conn.trace.despawn(render_cf, id);
        }
        EntityChange::Translate { id, translation } => {
            conn.trace.set_translation(render_cf, id, translation);

            cmd_buffer.push(Command::Translate {
                entity: id,
                start: render_cf,
                end: render_cf + 1,
                dst: translation,
            });
        }
        EntityChange::Rotate { id, rotation } => {
            conn.trace.set_rotation(render_cf, id, rotation);

            cmd_buffer.push(Command::Rotate {
                entity: id,
                start: render_cf,
                end: render_cf + 1,
                dst: rotation,
            });
        }
        EntityChange::CreateHost { id } => {
            cmd_buffer.push(Command::SpawnHost(id));
        }
        EntityChange::DestroyHost { id } => {
            cmd_buffer.push(Command::Despawn(id));
        }
        EntityChange::Health { id, health } => {
            // let entity = conn.entities.get(id).unwrap();

            // TODO
        }
        EntityChange::InventoryItemAdd(event) => {
            // let entity = conn.entities.get(event.entity).unwrap();

            // TODO
        }
        EntityChange::InventoryItemRemove(event) => {
            // let entity = conn.entities.get(event.entity).unwrap();

            // TODO
        }
        EntityChange::InventoryDestroy(event) => {
            // let entity = conn.entities.get(event.entity).unwrap();

            // TODO
        }
        EntityChange::CreateStreamingSource { id, source } => {}
        EntityChange::RemoveStreamingSource { id } => {}
    }
}

#[derive(Clone, Debug)]
pub struct DelayedEntity {
    pub entity: Entity,
    pub host: bool,
    pub inventory: Inventory,
}

impl From<Entity> for DelayedEntity {
    fn from(value: Entity) -> Self {
        Self {
            entity: value,
            host: false,
            inventory: Inventory::new(),
        }
    }
}

struct Buffer {
    entities: Vec<DelayedEntity>,
}

impl Buffer {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    pub fn push<E>(&mut self, entity: E)
    where
        E: Into<DelayedEntity>,
    {
        self.entities.push(entity.into());
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut DelayedEntity> {
        self.entities.iter_mut().find(|e| e.entity.id == id)
    }

    pub fn remove(&mut self, id: EntityId) -> bool {
        let mut removed = false;
        self.entities.retain(|e| {
            if e.entity.id != id {
                true
            } else {
                removed = true;
                false
            }
        });

        removed
    }
}

fn add_inventory_item(inventory: &mut Inventory, modules: &Modules, event: InventoryItemAdd) {
    let module = modules.get(event.item.0.module).unwrap();
    let record = module.records.get(event.item.0.record).unwrap();
    let item = record.clone().body.unwrap_item();

    let mut components = Components::new();
    for comp in item.components {
        components.insert(comp.record, components::Component { bytes: comp.value });
    }

    let mut actions = Actions::new();
    for action in item.actions {
        actions.push(ActionId(action));
    }

    let item = Item {
        id: event.item,
        resistances: None,
        mass: item.mass,
        actions,
        components,
        equipped: false,
        hidden: false,
    };

    if let Err(err) = inventory.insert(item) {
        tracing::error!("failed to insert item into inventory: {}", err);
    }
}

fn create_initial_diff(view: WorldViewRef) -> Vec<EntityChange> {
    let mut deltas = vec![];

    for entity in view.iter() {
        deltas.push(EntityChange::Create {
            entity: entity.clone(),
        });

        if entity.is_host {
            deltas.push(EntityChange::CreateHost { id: entity.id });
        }
    }

    deltas
}

fn create_snapshot_diff(prev: WorldViewRef, next: WorldViewRef) -> Vec<EntityChange> {
    let mut deltas = vec![];

    let mut visited_entities = vec![];

    for entity in prev.iter() {
        visited_entities.push(entity.id);

        let Some(next_entity) = next.get(entity.id) else {
            deltas.push(EntityChange::Destroy { id: entity.id });
            continue;
        };

        if entity.transform.translation != next_entity.transform.translation {
            deltas.push(EntityChange::Translate {
                id: entity.id,
                translation: next_entity.transform.translation,
            });
        }

        if entity.transform.rotation != next_entity.transform.rotation {
            deltas.push(EntityChange::Rotate {
                id: entity.id,
                rotation: next_entity.transform.rotation,
            });
        }

        match (entity.is_host, next_entity.is_host) {
            (true, true) | (false, false) => (),
            (false, true) => deltas.push(EntityChange::CreateHost { id: entity.id }),
            (true, false) => deltas.push(EntityChange::DestroyHost { id: entity.id }),
        }
    }

    for entity in next.iter().filter(|e| !visited_entities.contains(&e.id)) {
        deltas.push(EntityChange::Create {
            entity: entity.clone(),
        });

        if entity.is_host {
            deltas.push(EntityChange::CreateHost { id: entity.id });
        }
    }

    deltas
}

#[derive(Clone, Debug, Default)]
pub struct CommandBuffer {
    buffer: VecDeque<Command>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
        }
    }

    pub fn push(&mut self, cmd: Command) {
        self.buffer.push_back(cmd);
    }

    pub fn pop(&mut self) -> Option<Command> {
        self.buffer.pop_front()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    Spawn(DelayedEntity),
    Despawn(EntityId),
    Translate {
        entity: EntityId,
        start: ControlFrame,
        end: ControlFrame,
        dst: Vec3,
    },
    Rotate {
        entity: EntityId,
        start: ControlFrame,
        end: ControlFrame,
        dst: Quat,
    },
    SpawnHost(EntityId),
    DestroyHost(EntityId),
}

#[cfg(test)]
mod tests {
    use game_common::components::object::ObjectId;
    use game_common::components::transform::Transform;
    use game_common::entity::EntityId;
    use game_common::record::RecordReference;
    use game_common::world::control_frame::ControlFrame;
    use game_common::world::entity::{Entity, EntityBody, Object};
    use game_common::world::world::WorldState;

    use super::create_snapshot_diff;

    fn create_test_entity() -> Entity {
        Entity {
            id: EntityId::dangling(),
            transform: Transform::default(),
            body: EntityBody::Object(Object {
                id: ObjectId(RecordReference::STUB),
            }),
            components: Default::default(),
            is_host: false,
        }
    }

    #[test]
    fn create_diff_create() {
        let mut world = WorldState::new();
        world.insert(ControlFrame(0));
        world.insert(ControlFrame(1));

        let mut view = world.get_mut(ControlFrame(1)).unwrap();
        view.spawn(create_test_entity());
        drop(view);

        let prev = world.get(ControlFrame(0)).unwrap();
        let next = world.get(ControlFrame(1)).unwrap();
        let diff = create_snapshot_diff(prev, next);

        assert_eq!(diff.len(), 1);
    }
}
