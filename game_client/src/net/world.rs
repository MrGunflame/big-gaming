use std::time::{Duration, Instant};

use bevy_ecs::system::{Commands, Query, Res, ResMut};
use game_common::components::actions::{ActionId, Actions};
use game_common::components::actor::ActorProperties;
use game_common::components::combat::Health;
use game_common::components::components::{self, Components};
use game_common::components::inventory::Inventory;
use game_common::components::items::Item;
use game_common::components::player::HostPlayer;
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::world::entity::{Entity, EntityBody};
use game_common::world::snapshot::{EntityChange, InventoryItemAdd};
use game_common::world::source::StreamingSource;
use game_common::world::world::{WorldState, WorldViewRef};
use game_core::modules::Modules;
use game_input::hotkeys::Hotkeys;
use game_net::backlog::Backlog;
use game_net::snapshot::DeltaQueue;

use crate::entities::actor::LoadActor;
use crate::entities::inventory::{AddInventoryItem, DestroyInventory, RemoveInventoryItem};
use crate::entities::item::LoadItem;
use crate::entities::object::LoadObject;
use crate::entities::terrain::LoadTerrain;
use crate::plugins::actions::ActiveActions;

use super::ServerConnection;

pub fn apply_world_delta(
    mut world: ResMut<WorldState>,
    mut queue: ResMut<DeltaQueue>,
    mut conn: ResMut<ServerConnection>,
) {
    let conn = &mut *conn;

    let period = &mut conn.interpolation_period;

    // Don't start a new period until the previous ended.
    if period.end > Instant::now() - Duration::from_millis(100) {
        return;
    }

    if world.len() < 2 {
        return;
    }

    // Apply client-side prediction
    let view = world.at_mut(0).unwrap();
    conn.overrides.apply(view);
    // drop(view);

    let (Some(curr), Some(next)) = (world.at(0), world.at(1)) else {
        return;
    };

    debug_assert_ne!(curr.creation(), next.creation());

    // The end of the previous snapshot should be the current snapshot.
    if cfg!(debug_assertions) {
        // Ignore the start, where start == end.
        if period.start != period.end {
            assert_eq!(period.end, curr.creation());
        }
    }

    period.start = curr.creation();
    period.end = next.creation();

    let delta = WorldViewRef::delta(Some(curr), next);

    for change in delta {
        queue.push(change);
    }

    world.pop();
}

pub fn flush_delta_queue(
    mut commands: Commands,
    mut queue: ResMut<DeltaQueue>,
    mut entities: Query<(
        bevy_ecs::entity::Entity,
        &mut Transform,
        Option<&mut Health>,
        // FIXME: We prolly don't want this on entity directly and just
        // access the WorldState.
        Option<&mut Inventory>,
        Option<&mut ActorProperties>,
    )>,
    mut backlog: ResMut<Backlog>,
    conn: Res<ServerConnection>,
    modules: Res<Modules>,
    mut active_actions: ResMut<ActiveActions>,
    mut hotkeys: ResMut<Hotkeys>,
) {
    // Since events are received in batches, and commands are not applied until
    // the system is done, we buffer all created entities so we can modify them
    // in place within the same batch before they are spawned into the world.
    let mut buffer = Buffer::new();

    while let Some(event) = queue.pop() {
        handle_event(
            &mut commands,
            &mut entities,
            event,
            &mut buffer,
            &conn,
            &mut backlog,
            &modules,
        );
    }

    for entity in buffer.entities {
        let id = entity.entity.id;
        let entity = spawn_entity(&mut commands, entity);
        conn.entities.insert(id, entity);
    }
}

fn handle_event(
    commands: &mut Commands,
    entities: &mut Query<(
        bevy_ecs::entity::Entity,
        &mut Transform,
        Option<&mut Health>,
        Option<&mut Inventory>,
        Option<&mut ActorProperties>,
    )>,
    event: EntityChange,
    buffer: &mut Buffer,
    conn: &ServerConnection,
    backlog: &mut Backlog,
    modules: &Modules,
) {
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
        if conn.entities.get(entity_id).is_none() {
            if let Some(entity) = buffer.get_mut(entity_id) {
                match event {
                    EntityChange::Create { entity: _ } => {}
                    EntityChange::Destroy { id: _ } => {}
                    EntityChange::Translate {
                        id: _,
                        translation,
                        cell: _,
                    } => {
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
                    EntityChange::UpdateStreamingSource { id, state } => todo!(),
                    EntityChange::InventoryItemAdd(event) => {
                        add_inventory_item(&mut entity.inventory, modules, event);
                    }
                    EntityChange::InventoryItemRemove(event) => {
                        entity.inventory.remove(event.id);
                    }
                    EntityChange::InventoryDestroy(event) => {
                        entity.inventory.clear();
                    }
                }
            } else {
                backlog.push(entity_id, event);
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
            let Some(entity) = conn.entities.get(id) else {
                tracing::warn!("attempted to destroy a non-existent entity: {:?}", id);
                return;
            };

            if !buffer.remove(id) {
                commands.entity(entity).despawn();
            }
        }
        EntityChange::Translate {
            id,
            translation,
            cell,
        } => {
            let entity = conn.entities.get(id).unwrap();

            if let Ok((_, mut transform, _, _, _)) = entities.get_mut(entity) {
                transform.translation = translation;
            }
        }
        EntityChange::Rotate { id, rotation } => {
            let entity = conn.entities.get(id).unwrap();

            if let Ok((_, _, _, _, Some(mut props))) = entities.get_mut(entity) {
                props.rotation = rotation;
            }
        }
        EntityChange::CreateHost { id } => {
            let entity = conn.entities.get(id).unwrap();

            commands
                .entity(entity)
                .insert(HostPlayer)
                .insert(StreamingSource::new());
        }
        EntityChange::DestroyHost { id } => {
            let entity = conn.entities.get(id).unwrap();

            commands
                .entity(entity)
                .remove::<HostPlayer>()
                .remove::<StreamingSource>();
        }
        EntityChange::Health { id, health } => {
            let entity = conn.entities.get(id).unwrap();

            if let Ok((_, _, Some(mut h), _, _)) = entities.get_mut(entity) {
                *h = health;
            }
        }
        EntityChange::UpdateStreamingSource { id, state } => todo!(),
        EntityChange::InventoryItemAdd(event) => {
            let entity = conn.entities.get(event.entity).unwrap();

            commands.spawn(AddInventoryItem {
                entity,
                slot: event.id,
                id: event.item,
            });
        }
        EntityChange::InventoryItemRemove(event) => {
            let entity = conn.entities.get(event.entity).unwrap();

            commands.spawn(RemoveInventoryItem {
                entity,
                slot: event.id,
            });
        }
        EntityChange::InventoryDestroy(event) => {
            let entity = conn.entities.get(event.entity).unwrap();

            commands.spawn(DestroyInventory { entity });
        }
    }
}

fn spawn_entity(commands: &mut Commands, entity: DelayedEntity) -> bevy_ecs::entity::Entity {
    match entity.entity.body {
        EntityBody::Terrain(terrain) => commands.spawn(LoadTerrain { mesh: terrain }).id(),
        EntityBody::Object(object) => commands
            .spawn(LoadObject {
                transform: entity.entity.transform,
                id: object.id,
            })
            .id(),
        EntityBody::Actor(actor) => commands
            .spawn(LoadActor {
                transform: entity.entity.transform,
                race: actor.race,
                health: actor.health,
                host: entity.host,
                inventory: entity.inventory,
            })
            .id(),
        EntityBody::Item(item) => commands
            .spawn(LoadItem {
                transform: entity.entity.transform,
                id: item.id,
            })
            .id(),
    }

    // match &entity.entity.body {
    //     EntityBody::Terrain(terrain) => {
    //         let id = commands
    //             .spawn(LoadTerrain {
    //                 cell: terrain.cell,
    //                 mesh: terrain.clone(),
    //             })
    //             .insert(TransformBundle {
    //                 local: entity.entity.transform,
    //                 global: Default::default(),
    //             })
    //             .insert(VisibilityBundle::new())
    //             .insert(entity.entity)
    //             .id();

    //         id
    //     }
    //     EntityBody::Object(object) => {
    //         let id = commands
    //             .spawn(
    //                 ObjectBundle::new(object.id)
    //                     .translation(entity.entity.transform.translation)
    //                     .rotation(entity.entity.transform.rotation),
    //             )
    //             .insert(entity.entity)
    //             .insert(VisibilityBundle::new())
    //             .id();

    //         id
    //     }
    //     EntityBody::Actor(act) => {
    //         let mut actor = ActorBundle::default();
    //         actor.transform.transform.translation = entity.entity.transform.translation;
    //         actor.transform.transform.rotation = entity.entity.transform.rotation;
    //         actor.combat.health = act.health;

    //         actor.properties.eyes = Vec3::new(0.0, 1.6, -0.1);

    //         let mut cmds = commands.spawn(actor);
    //         cmds.insert(entity.entity);
    //         Human::default().spawn(assets, &mut cmds);

    //         if entity.host {
    //             cmds.insert(HostPlayer)
    //                 .insert(StreamingSource::new())
    //                 .insert(entity.inventory).insert(VisibilityBundle::new());
    //         }

    //         cmds.id()
    //     }
    //     EntityBody::Item(item) => {
    //         let id = commands
    //             .spawn(LoadItem::new(item.id))
    //             .insert(TransformBundle {
    //                 local: entity.entity.transform,
    //                 global: Default::default(),
    //             })
    //             .insert(VisibilityBundle::new())
    //             .insert(entity.entity)
    //             .id();

    //         id
    //     }
    // }
}

#[derive(Clone, Debug)]
struct DelayedEntity {
    entity: Entity,
    host: bool,
    inventory: Inventory,
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
