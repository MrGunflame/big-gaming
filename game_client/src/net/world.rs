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
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::{Entity, EntityBody};
use game_common::world::snapshot::{EntityChange, InventoryItemAdd};
use game_common::world::source::StreamingSource;
use game_common::world::world::{WorldState, WorldViewRef};
use game_core::counter::Interval;
use game_core::modules::Modules;

use crate::entities::actor::LoadActor;
use crate::entities::inventory::{AddInventoryItem, DestroyInventory, RemoveInventoryItem};
use crate::entities::item::LoadItem;
use crate::entities::object::LoadObject;
use crate::entities::terrain::LoadTerrain;
use crate::net::interpolate::{InterpolateRotation, InterpolateTranslation};

use super::ServerConnection;

pub fn apply_world_delta(
    mut conn: ResMut<ServerConnection<Interval>>,
    mut commands: Commands,
    mut entities: Query<(
        bevy_ecs::entity::Entity,
        &mut Transform,
        Option<&mut Health>,
        // FIXME: We prolly don't want this on entity directly and just
        // access the WorldState.
        Option<&mut Inventory>,
        Option<&mut ActorProperties>,
        &mut InterpolateTranslation,
        &mut InterpolateRotation,
    )>,
    modules: Res<Modules>,
) {
    let conn = &mut *conn;

    let cf = conn.control_frame();

    // Don't start rendering if the initial interpoation window is not
    // yet filled.
    let Some(render_cf) = cf.render else {
        return;
    };

    if conn.last_render_frame == render_cf {
        return;
    }

    debug_assert!(conn.world.len() >= 2);

    let prev = conn.world.at(0).unwrap();
    let next = conn.world.at(1).unwrap();

    let delta = create_snapshot_diff(prev, next);

    // Since events are received in batches, and commands are not applied until
    // the system is done, we buffer all created entities so we can modify them
    // in place within the same batch before they are spawned into the world.
    let mut buffer = Buffer::new();

    for event in delta {
        handle_event(
            &mut commands,
            &mut entities,
            event.clone(),
            &mut buffer,
            conn,
            &modules,
            render_cf,
        );
    }

    for entity in buffer.entities {
        conn.trace.spawn(render_cf, entity.entity.clone());

        let id = entity.entity.id;
        let entity = spawn_entity(&mut commands, entity);
        conn.entities.insert(id, entity);
    }

    conn.world.pop();
    conn.last_render_frame = render_cf;
}

fn handle_event(
    commands: &mut Commands,
    entities: &mut Query<(
        bevy_ecs::entity::Entity,
        &mut Transform,
        Option<&mut Health>,
        Option<&mut Inventory>,
        Option<&mut ActorProperties>,
        &mut InterpolateTranslation,
        &mut InterpolateRotation,
    )>,
    event: EntityChange,
    buffer: &mut Buffer,
    conn: &mut ServerConnection<Interval>,
    modules: &Modules,
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
        if conn.entities.get(entity_id).is_none() {
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
                        add_inventory_item(&mut entity.inventory, modules, event);
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
            } else {
                conn.backlog.push(entity_id, event);
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

            conn.trace.despawn(render_cf, id);
        }
        EntityChange::Translate { id, translation } => {
            let entity = conn.entities.get(id).unwrap();

            conn.trace.set_translation(render_cf, id, translation);

            if let Ok((_, transform, _, _, _, mut interpolate, _)) = entities.get_mut(entity) {
                // Translation is predicted, do not interpolate.
                // if let Some(translation) = conn
                //     .overrides
                //     .get_entity(id)
                //     .map(|p| p.translation())
                //     .flatten()
                // {
                //     // Predictected values should already be applied.
                //     // if cfg!(debug_assertions) {
                //     //     assert_eq!(transform.translation, translation);
                //     // }

                //     return;
                // }

                dbg!(translation);

                let translation = conn
                    .predictions
                    .get_translation(view, id)
                    .unwrap_or(translation);
                dbg!(transform.translation, translation);

                interpolate.set(transform.translation, translation, render_cf, render_cf + 1);
            }
        }
        EntityChange::Rotate { id, rotation } => {
            let entity = conn.entities.get(id).unwrap();

            conn.trace.set_rotation(render_cf, id, rotation);

            if let Ok((_, mut transform, _, _, props, _, mut interpolate)) =
                entities.get_mut(entity)
            {
                // Rotation is predicted, do not interpolate.
                // if let Some(rotation) = conn
                //     .predictions
                //     .get_entity(id)
                //     .map(|p| p.rotation())
                //     .flatten()
                // {
                //     // Predictected values should already be applied.
                //     // if cfg!(debug_assertions) {
                //     //     if let Some(props) = props {
                //     //         assert_eq!(props.rotation, rotation);
                //     //     } else {
                //     //         assert_eq!(transform.rotation, rotation);
                //     //     }
                //     // }

                //     return;
                // }

                let rot = conn.predictions.get_rotation(id).unwrap_or(rotation);

                if let Some(props) = props {
                    interpolate.set(props.rotation, rot, render_cf, render_cf + 1);
                } else {
                    interpolate.set(transform.rotation, rot, render_cf, render_cf + 1);
                }

                // transform.rotation = rotation;
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

            if let Ok((_, _, Some(mut h), _, _, _, _)) = entities.get_mut(entity) {
                *h = health;
            }
        }
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
        EntityChange::CreateStreamingSource { id, source } => {}
        EntityChange::RemoveStreamingSource { id } => {}
    }
}

fn spawn_entity(commands: &mut Commands, entity: DelayedEntity) -> bevy_ecs::entity::Entity {
    match entity.entity.body {
        EntityBody::Terrain(terrain) => commands.spawn(LoadTerrain { terrain }).id(),
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
