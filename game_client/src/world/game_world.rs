use std::time::{Duration, Instant};

use game_common::components::actions::ActionId;
use game_common::components::Transform;
use game_common::entity::EntityId;
use game_common::events::{ActionEvent, Event, EventQueue};
use game_common::net::ServerEntity;
use game_common::world::control_frame::ControlFrame;
use game_common::world::hierarchy::update_global_transform;
use game_core::counter::{Interval, UpdateCounter};
use game_core::modules::Modules;
use game_net::message::{DataMessageBody, EntityAction};
use game_net::peer_error;
use game_script::Executor;
use game_tracing::trace_span;
use game_wasm::resource::RuntimeResourceId;

use crate::config::Config;
use crate::net::world::{Command, CommandBuffer};
use crate::net::{Entities, ServerConnection};
use crate::world::script::run_scripts;

use super::state::WorldState;
use super::RemoteError;

// The maximum number of update cycles allowed per frame. This prevents situations
// where the update takes longer than the frame and therefore causes the game loop
// to fall even further behind and never return.
const MAX_UPDATES_PER_FRAME: u32 = 10;

const DRIFT_RESYNC_DURATION: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub struct GameWorld {
    conn: ServerConnection,
    pub(crate) game_tick: GameTick,
    next_frame_counter: NextFrameCounter,
    /// Server to local entity mapping.
    server_entities: Entities,
    physics_pipeline: game_physics::Pipeline,
    event_queue: EventQueue,

    /// Newest fresh state from the server.
    newest_state: WorldState,
    /// The newest state from the server with locally predicted inputs applied.
    predicted_state: WorldState,

    interval: Interval,
    server_tick_rate: ServerTickRate,
}

impl GameWorld {
    pub fn new(conn: ServerConnection, config: &Config) -> Self {
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
            event_queue: EventQueue::new(),
            predicted_state: WorldState::new(),
            interval: Interval::new(Duration::from_secs(1) / config.timestep),
            server_tick_rate: ServerTickRate::new(config.timestep),
        }
    }

    pub fn rtt(&self) -> Duration {
        self.conn.rtt()
    }

    pub async fn update(
        &mut self,
        modules: &Modules,
        executor: &mut Executor,
        cmd_buffer: &mut CommandBuffer,
    ) -> Result<(), RemoteError> {
        if !self.conn.is_connected() {
            return Err(RemoteError::Disconnected);
        }

        let now = Instant::now();
        self.interval.wait(now).await;

        let _span = trace_span!("GameWorld::update").entered();

        self.conn.update();
        self.server_tick_rate.update(now, self.conn.latest_cf);

        // The drift value is the relative distance between our control frame and the server's
        // control frame. Since the server only sends periodic ACKs we need to account for RTT
        // when computing the server's control frame.
        // Drift is positive if we are ahead of the server and negative if we are behind.
        let server_cf = self.server_tick_rate.predict_frame(now, self.rtt());
        let drift = i32::from(self.game_tick.current_control_frame.0) - i32::from(server_cf.0);

        // To keep the client in sync with the server we need to dynamically adjust
        // our timestep to slow down/speed up as the server does.
        // To reach the exact control frame of the server we compute an additional
        // time compensation value that allows us the catch up to the server.
        let compensation = compute_compensation(&self.server_tick_rate, drift);
        if drift.is_positive() {
            let server_timestep = self.server_tick_rate.frame_time;
            let timestep = server_timestep.saturating_add(compensation);
            self.interval.set_timestep(timestep);
        } else {
            let server_timestep = self.server_tick_rate.frame_time;
            let timestep = server_timestep.saturating_sub(compensation);
            self.interval.set_timestep(timestep);
        }

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
                executor,
                &mut self.event_queue,
                &modules,
            );

            update_global_transform(&mut self.predicted_state.world);
        }

        self.next_frame_counter.update();
        self.conn.set_cf(self.game_tick.current_control_frame);

        Ok(())
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
                    }
                    DataMessageBody::SpawnHost(msg) => {
                        let Some(id) = self.server_entities.get(msg.entity) else {
                            peer_error!("invalid entity: {:?}", msg.entity);
                            continue;
                        };

                        cmd_buffer.push(Command::SpawnHost(id));
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
                    }
                    DataMessageBody::EntityComponentRemove(msg) => {
                        let Some(id) = self.server_entities.get(msg.entity) else {
                            peer_error!("invalid entity: {:?}", msg.entity);
                            continue;
                        };

                        self.newest_state.world.remove(id, msg.component);
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
                    DataMessageBody::ResourceCreate(msg) => {
                        let id = RuntimeResourceId::from_bits(msg.id.0);
                        self.newest_state
                            .world
                            .insert_resource_with_id(msg.data.into(), id);
                    }
                    DataMessageBody::ResourceDestroy(msg) => {
                        let id = RuntimeResourceId::from_bits(msg.id.0);
                        self.newest_state.world.remove_resource(id);
                    }
                }
            }
        }

        self.apply_predicted_inputs(cf);
    }

    fn apply_predicted_inputs(&mut self, cf: ControlFrame) {
        // Remove all inputs that were acknowledged for this frame
        // BEFORE we apply them.
        self.conn.input_buffer.clear(cf);

        // We need to replicate the world snapshot as the client
        // predicted it.
        self.predicted_state = self.newest_state.clone();

        for msg in self.conn.input_buffer.iter() {
            match &msg.body {
                DataMessageBody::EntityTranslate(msg) => {
                    let id = self.server_entities.get(msg.entity).unwrap();
                    let mut transform: Transform =
                        self.predicted_state.world.get_typed(id).unwrap();
                    transform.translation = msg.translation;
                    self.predicted_state.world.insert_typed(id, transform);
                }
                DataMessageBody::EntityRotate(msg) => {
                    let id = self.server_entities.get(msg.entity).unwrap();
                    let mut transform: Transform =
                        self.predicted_state.world.get_typed(id).unwrap();
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

#[derive(Clone, Debug)]
struct ServerTickRate {
    last_update: Instant,
    last_cf: ControlFrame,
    frame_time: Duration,
}

impl ServerTickRate {
    fn new(timestep: u32) -> Self {
        Self {
            last_update: Instant::now(),
            last_cf: ControlFrame(0),
            frame_time: Duration::from_secs(1) / timestep,
        }
    }

    fn update(&mut self, now: Instant, cf: ControlFrame) {
        if cf == self.last_cf {
            return;
        }

        let delta_cf = cf - self.last_cf;
        let delta = now - self.last_update;

        self.last_update = now;
        self.last_cf = cf;

        let Some(elapsed_per_cf) = delta.checked_div(u32::from(delta_cf.0)) else {
            return;
        };

        self.frame_time = self.frame_time.mul_f32(0.8) + elapsed_per_cf.mul_f32(0.2);
    }

    fn predict_frame(&self, now: Instant, rtt: Duration) -> ControlFrame {
        let mut delta = (now - self.last_update) + rtt / 2;
        let mut cf = self.last_cf;
        while let Some(ts) = delta.checked_sub(self.frame_time) {
            delta = ts;
            cf += 1;
        }
        cf
    }
}

/// Computes the timestep compensation for the given `drift` value.
///
/// The returned `Duration` should be added to the per-frame timestep to compensate for the given
/// `drift` within [`DRIFT_RESYNC_DURATION`].
fn compute_compensation(server_tick_rate: &ServerTickRate, drift: i32) -> Duration {
    let catchup_time = server_tick_rate.frame_time * drift.unsigned_abs();
    let ups =
        (DRIFT_RESYNC_DURATION.as_secs_f64() / server_tick_rate.frame_time.as_secs_f64()) as u32;
    catchup_time.checked_div(ups).unwrap_or_default()
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use super::{compute_compensation, ServerTickRate};

    #[test]
    fn test_compute_compensation() {
        let tick_rate = ServerTickRate::new(50);
        let drift = 5;

        let output = compute_compensation(&tick_rate, drift);
        assert_eq!(output, Duration::from_millis(2));
    }
}
