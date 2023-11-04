use ahash::HashMap;
use game_common::components::components::{Component, Components};
use game_common::components::object::ObjectId;
use game_common::components::race::RaceId;
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::events::EventQueue;
use game_common::net::ServerEntity;
use game_common::record::RecordReference;
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::{Actor, Entity, EntityBody, Object};
use game_core::counter::{IntervalImpl, UpdateCounter};
use game_core::modules::Modules;
use game_core::time::Time;
use game_data::record::RecordBody;
use game_net::message::DataMessageBody;
use game_net::peer_error;
use game_script::executor::ScriptExecutor;
use game_tracing::trace_span;

use crate::net::world::{Command, CommandBuffer};
use crate::net::ServerConnection;
use crate::world::script::run_scripts;

use super::state::WorldState;

#[derive(Debug)]
pub struct GameWorld<I> {
    conn: ServerConnection,
    game_tick: GameTick<I>,
    next_frame_counter: NextFrameCounter,
    /// Server to local entity mapping.
    server_entities: HashMap<ServerEntity, EntityId>,
    state: WorldState,
    physics_pipeline: game_physics::Pipeline,
    executor: ScriptExecutor,
    event_queue: EventQueue,
}

impl<I> GameWorld<I>
where
    I: IntervalImpl,
{
    pub fn new(conn: ServerConnection, interval: I, executor: ScriptExecutor) -> Self {
        Self {
            conn,
            game_tick: GameTick {
                interval,
                counter: UpdateCounter::new(),
                current_control_frame: ControlFrame(0),
            },
            state: WorldState::new(),
            server_entities: HashMap::default(),
            next_frame_counter: NextFrameCounter::new(ControlFrame(0)),
            physics_pipeline: game_physics::Pipeline::new(),
            executor,
            event_queue: EventQueue::new(),
        }
    }

    pub fn update(&mut self, time: &Time, modules: &Modules, cmd_buffer: &mut CommandBuffer) {
        let _span = trace_span!("GameWorld::update").entered();

        while self.game_tick.interval.is_ready(time.last_update()) {
            self.conn.update2();

            self.game_tick.current_control_frame += 1;
            self.game_tick.counter.update();

            tracing::debug!(
                "Stepping control frame to {:?} (UPS = {})",
                self.game_tick.current_control_frame,
                self.game_tick.counter.ups(),
            );

            if let Some(render_cf) = self.next_frame_counter.render_frame {
                self.process_frame(self.game_tick.current_control_frame, modules, cmd_buffer);

                run_scripts(
                    &mut self.state,
                    &self.physics_pipeline,
                    &self.executor,
                    &mut self.event_queue,
                    cmd_buffer,
                );
            }

            self.next_frame_counter.update();
        }
    }

    fn process_frame(
        &mut self,
        cf: ControlFrame,
        modules: &Modules,
        cmd_buffer: &mut CommandBuffer,
    ) {
        let _span = trace_span!("GameWorld::process_frame").entered();

        // If we didn't receive any messages in this CF this is `None`
        // and we don't have to do anything.
        let Some(iter) = self.conn.backlog.drain(cf) else {
            return;
        };

        for msg in iter {
            match msg.body {
                DataMessageBody::EntityCreate(msg) => {
                    let id = match msg.data {
                        EntityBody::Actor(actor) => actor.race.0,
                        EntityBody::Object(object) => object.id.0,
                        _ => todo!(),
                    };

                    let entity = spawn_entity(
                        id,
                        Transform {
                            translation: msg.translation,
                            rotation: msg.rotation,
                            ..Default::default()
                        },
                        modules,
                    );

                    let id = self.state.entities.insert(entity);
                    self.server_entities.insert(msg.entity, id);

                    cmd_buffer.push(Command::Spawn(entity));
                }
                DataMessageBody::EntityDestroy(msg) => {
                    let Some(id) = self.server_entities.get(&msg.entity).copied() else {
                        peer_error!("invalid entity: {:?}", msg.entity);
                        continue;
                    };

                    self.state.entities.remove(id);

                    cmd_buffer.push(Command::Despawn(id));
                }
                DataMessageBody::EntityTranslate(msg) => {
                    let Some(id) = self.server_entities.get(&msg.entity).copied() else {
                        peer_error!("invalid entity: {:?}", msg.entity);
                        continue;
                    };

                    self.state
                        .entities
                        .get_mut(id)
                        .unwrap()
                        .transform
                        .translation = msg.translation;

                    cmd_buffer.push(Command::Translate {
                        entity: id,
                        dst: msg.translation,
                    });
                }
                DataMessageBody::EntityRotate(msg) => {
                    let Some(id) = self.server_entities.get(&msg.entity).copied() else {
                        peer_error!("invalid entity: {:?}", msg.entity);
                        continue;
                    };

                    self.state.entities.get_mut(id).unwrap().transform.rotation = msg.rotation;

                    cmd_buffer.push(Command::Rotate {
                        entity: id,
                        dst: msg.rotation,
                    });
                }
                _ => todo!(),
            }
        }
    }
}

#[derive(Debug)]
pub struct GameTick<I> {
    pub interval: I,
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

fn spawn_entity(id: RecordReference, transform: Transform, modules: &Modules) -> Option<Entity> {
    let Some(module) = modules.get(id.module) else {
        return None;
    };

    let Some(record) = module.records.get(id.record) else {
        return None;
    };

    let body = match &record.body {
        RecordBody::Race(race) => EntityBody::Actor(Actor { race: RaceId(id) }),
        RecordBody::Object(object) => EntityBody::Object(Object { id: ObjectId(id) }),
        _ => todo!(),
    };

    let mut components = Components::new();
    for component in &record.components {
        components.insert(
            component.id,
            Component {
                bytes: component.bytes.clone(),
            },
        );
    }

    Some(Entity {
        id: EntityId::dangling(),
        transform,
        body,
        components,
        is_host: false,
    })
}
