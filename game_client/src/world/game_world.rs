use game_common::components::actions::ActionId;
use game_common::components::items::{Item, ItemStack};
use game_common::components::Transform;
use game_common::entity::EntityId;
use game_common::events::{ActionEvent, Event, EventQueue};
use game_common::net::ServerEntity;
use game_common::units::Mass;
use game_common::world::control_frame::ControlFrame;
use game_core::counter::UpdateCounter;
use game_core::modules::Modules;
use game_net::message::{DataMessageBody, EntityAction};
use game_net::peer_error;
use game_script::Executor;
use game_tracing::trace_span;

use crate::config::Config;
use crate::net::world::{Command, CommandBuffer};
use crate::net::{Entities, ServerConnection};
use crate::world::script::run_scripts;

use super::state::WorldState;

// The maximum number of update cycles allowed per frame. This prevents situations
// where the update takes longer than the frame and therefore causes the game loop
// to fall even further behind and never return.
const MAX_UPDATES_PER_FRAME: u32 = 10;

#[derive(Debug)]
pub struct GameWorld {
    conn: ServerConnection,
    pub(crate) game_tick: GameTick,
    next_frame_counter: NextFrameCounter,
    /// Server to local entity mapping.
    server_entities: Entities,
    physics_pipeline: game_physics::Pipeline,
    executor: Executor,
    event_queue: EventQueue,

    /// Newest fresh state from the server.
    newest_state: WorldState,
    /// The newest state from the server with locally predicted inputs applied.
    predicted_state: WorldState,
}

impl GameWorld {
    pub fn new(conn: ServerConnection, executor: Executor, config: &Config) -> Self {
        let render_delay = ControlFrame(config.network.interpolation_frames);

        Self {
            conn,
            game_tick: GameTick {
                counter: UpdateCounter::new(),
                current_control_frame: ControlFrame(0),
            },
            newest_state: WorldState::new(),
            server_entities: Entities::default(),
            next_frame_counter: NextFrameCounter::new(render_delay),
            physics_pipeline: game_physics::Pipeline::new(),
            executor,
            event_queue: EventQueue::new(),
            predicted_state: WorldState::new(),
        }
    }

    pub fn update(&mut self, modules: &Modules, cmd_buffer: &mut CommandBuffer) {
        let _span = trace_span!("GameWorld::update").entered();

        self.conn.update();

        self.game_tick.current_control_frame += 1;
        self.game_tick.counter.update();

        tracing::debug!(
            "Stepping control frame to {:?} (UPS = {})",
            self.game_tick.current_control_frame,
            self.game_tick.counter.ups(),
        );

        if let Some(render_cf) = self.next_frame_counter.render_frame {
            self.process_frame(render_cf, cmd_buffer);

            run_scripts(
                &mut self.predicted_state,
                &self.physics_pipeline,
                &mut self.executor,
                &mut self.event_queue,
                &modules,
            );
        }

        self.next_frame_counter.update();
    }

    pub fn ups(&self) -> UpdateCounter {
        self.game_tick.counter.clone()
    }

    fn process_frame(&mut self, cf: ControlFrame, cmd_buffer: &mut CommandBuffer) {
        let _span = trace_span!("GameWorld::process_frame").entered();

        // If we didn't receive any messages in this CF this is `None`
        // but we still have to handle predicted inputs for this frame.
        if let Some(iter) = self.conn.backlog.drain(cf) {
            for msg in iter {
                match msg.body {
                    DataMessageBody::EntityDestroy(msg) => {
                        let Some(id) = self.server_entities.get(msg.entity) else {
                            peer_error!("invalid entity: {:?}", msg.entity);
                            continue;
                        };

                        self.newest_state.world.despawn(id);

                        cmd_buffer.push(Command::Despawn(id));
                    }
                    DataMessageBody::SpawnHost(msg) => {
                        let Some(id) = self.server_entities.get(msg.entity) else {
                            peer_error!("invalid entity: {:?}", msg.entity);
                            continue;
                        };

                        cmd_buffer.push(Command::SpawnHost(id));
                    }
                    DataMessageBody::InventoryItemAdd(msg) => {
                        let Some(id) = self.server_entities.get(msg.entity) else {
                            peer_error!("invalid entity: {:?}", msg.entity);
                            continue;
                        };

                        if self.newest_state.inventories.get(id).is_none() {
                            self.newest_state.inventories.insert(id);
                        }

                        let inventory = self.newest_state.inventories.get_mut(id).unwrap();
                        inventory
                            .insert_at_slot(
                                msg.id,
                                ItemStack {
                                    item: Item {
                                        id: msg.item,
                                        mass: Mass::default(),
                                        components: msg.components,
                                        equipped: msg.equipped,
                                        hidden: msg.hidden,
                                    },
                                    quantity: msg.quantity,
                                },
                            )
                            .unwrap();

                        if msg.equipped {
                            cmd_buffer.push(Command::InventoryItemEquip {
                                entity: id,
                                slot: msg.id,
                            });
                        }
                    }
                    DataMessageBody::InventoryItemRemove(msg) => {
                        let Some(id) = self.server_entities.get(msg.entity) else {
                            peer_error!("invalid entity: {:?}", msg.entity);
                            continue;
                        };

                        let inventory = self.newest_state.inventories.get_mut(id).unwrap();
                        if let Some(item) = inventory.remove(msg.slot, u32::MAX) {
                            if item.equipped {
                                cmd_buffer.push(Command::InventoryItemUnequip {
                                    entity: id,
                                    slot: msg.slot,
                                });
                            }
                        }
                    }
                    DataMessageBody::InventoryItemUpdate(msg) => {
                        let Some(id) = self.server_entities.get(msg.entity) else {
                            peer_error!("invalid entity: {:?}", msg.entity);
                            continue;
                        };

                        let inventory = self.newest_state.inventories.get_mut(id).unwrap();
                        let Some(stack) = inventory.get_mut(msg.slot) else {
                            peer_error!("invalid inventory slot: {:?}", msg.slot);
                            continue;
                        };

                        // Check whether if the actions of the stack may have changed.
                        // This happens when the items component changes and the item
                        // is equipped.
                        match (stack.item.equipped, msg.equipped, &msg.components) {
                            // 1. The item is not equipped, or component haven't changed.
                            (true, true, None) | (false, false, _) => (),
                            // 2. The item was equipped or the components have changed.
                            (true, true, Some(_)) | (false, true, _) => {
                                cmd_buffer.push(Command::InventoryItemEquip {
                                    entity: id,
                                    slot: msg.slot,
                                });
                            }
                            // 3. The item was uneqipped.
                            (true, false, _) => {
                                cmd_buffer.push(Command::InventoryItemUnequip {
                                    entity: id,
                                    slot: msg.slot,
                                });
                            }
                        }

                        stack.item.hidden = msg.hidden;
                        stack.item.equipped = msg.equipped;

                        if let Some(quantity) = msg.quantity {
                            stack.quantity = quantity;
                        }

                        if let Some(components) = msg.components {
                            stack.item.components = components;
                        }
                    }
                    DataMessageBody::EntityComponentAdd(msg) => {
                        let id = match self.server_entities.get(msg.entity) {
                            Some(id) => id,
                            None => {
                                let entity = self.newest_state.world.spawn();
                                self.server_entities.insert(entity, msg.entity);
                                entity
                            }
                        };

                        let component = msg
                            .component
                            .remap(|entity| {
                                let server_entity = ServerEntity(entity.into_raw());
                                match self.server_entities.get(server_entity) {
                                    Some(id) => Some(id),
                                    None => {
                                        let entity = self.newest_state.world.spawn();
                                        self.server_entities.insert(entity, server_entity);
                                        Some(entity)
                                    }
                                }
                            })
                            .unwrap();

                        self.newest_state
                            .world
                            .insert(id, msg.component_id, component);

                        cmd_buffer.push(Command::ComponentAdd {
                            entity: id,
                            component: msg.component_id,
                        });
                    }
                    DataMessageBody::EntityComponentRemove(msg) => {
                        let Some(id) = self.server_entities.get(msg.entity) else {
                            peer_error!("invalid entity: {:?}", msg.entity);
                            continue;
                        };

                        self.newest_state.world.remove(id, msg.component);

                        cmd_buffer.push(Command::ComponentRemove {
                            entity: id,
                            component: msg.component,
                        });
                    }
                    DataMessageBody::EntityComponentUpdate(msg) => {
                        let Some(id) = self.server_entities.get(msg.entity) else {
                            peer_error!("invalid entity: {:?}", msg.entity);
                            continue;
                        };

                        let component = msg
                            .component
                            .remap(|entity| {
                                let server_entity = ServerEntity(entity.into_raw());
                                match self.server_entities.get(server_entity) {
                                    Some(id) => Some(id),
                                    None => {
                                        let entity = self.newest_state.world.spawn();
                                        self.server_entities.insert(entity, server_entity);
                                        Some(entity)
                                    }
                                }
                            })
                            .unwrap();

                        self.newest_state
                            .world
                            .insert(id, msg.component_id, component);
                    }
                    DataMessageBody::EntityAction(msg) => todo!(),
                    DataMessageBody::EntityTranslate(_) | DataMessageBody::EntityRotate(_) => {
                        todo!()
                    }
                }
            }
        }

        // Apply predicted inputs.

        // Remove all inputs that were acknowledged for this frame
        // BEFORE we apply them.
        self.conn.input_buffer.clear(cf);

        for msg in self.conn.input_buffer.iter() {
            match &msg.body {
                DataMessageBody::EntityTranslate(msg) => {
                    let id = self.server_entities.get(msg.entity).unwrap();
                    cmd_buffer.push(Command::Translate {
                        entity: id,
                        dst: msg.translation,
                    });
                }
                DataMessageBody::EntityRotate(msg) => {
                    let id = self.server_entities.get(msg.entity).unwrap();
                    cmd_buffer.push(Command::Rotate {
                        entity: id,
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

        // We need to replicate the world snapshot as the client
        // predicted it.
        self.predicted_state = self.newest_state.clone();

        for msg in self.conn.input_buffer.iter() {
            match &msg.body {
                DataMessageBody::EntityTranslate(msg) => {
                    let id = self.server_entities.get(msg.entity).unwrap();
                    let mut transform: Transform = self.predicted_state.world.get_typed(id);
                    transform.translation = msg.translation;
                    self.predicted_state.world.insert_typed(id, transform);
                }
                DataMessageBody::EntityRotate(msg) => {
                    let id = self.server_entities.get(msg.entity).unwrap();
                    let mut transform: Transform = self.predicted_state.world.get_typed(id);
                    transform.rotation = msg.rotation;
                    self.predicted_state.world.insert_typed(id, transform);
                }
                DataMessageBody::EntityAction(msg) => {
                    let id = self.server_entities.get(msg.entity).unwrap();
                    self.event_queue.push(Event::Action(ActionEvent {
                        entity: id,
                        invoker: id,
                        action: msg.action,
                        data: msg.bytes.clone(),
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
    }

    pub fn state(&self) -> &WorldState {
        &self.predicted_state
    }

    pub fn state_mut(&mut self) -> &mut WorldState {
        &mut self.predicted_state
    }

    pub fn send(&mut self, action: Action) {
        let Some(id) = self.server_entities.get(action.entity) else {
            return;
        };

        self.conn.send(
            self.next_frame_counter.newest_frame,
            DataMessageBody::EntityAction(EntityAction {
                entity: id,
                action: action.action,
                bytes: action.data,
            }),
        );
    }

    pub fn input_buffer_len(&self) -> usize {
        self.conn.input_buffer.len()
    }
}

#[derive(Debug)]
pub struct GameTick {
    current_control_frame: ControlFrame,
    counter: UpdateCounter,
}

#[derive(Clone, Debug)]
struct NextFrameCounter {
    render_frame: Option<ControlFrame>,
    newest_frame: ControlFrame,
    render_delay: ControlFrame,
}

impl NextFrameCounter {
    fn new(render_delay: ControlFrame) -> Self {
        let render_frame = if render_delay.0 == 0 {
            Some(ControlFrame(0))
        } else {
            None
        };

        Self {
            render_delay,
            render_frame,
            newest_frame: ControlFrame(0),
        }
    }

    fn update(&mut self) {
        self.newest_frame += 1;

        if let Some(cf) = &mut self.render_frame {
            *cf += 1;
        } else {
            self.render_frame = self.newest_frame.checked_sub(self.render_delay);
        }
    }
}

#[derive(Clone, Debug)]
pub struct Action {
    pub entity: EntityId,
    pub action: ActionId,
    pub data: Vec<u8>,
}
