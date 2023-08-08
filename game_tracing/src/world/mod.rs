use std::time::SystemTime;

use game_common::entity::EntityId;
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::Entity;
use glam::{Quat, Vec3};

#[derive(Clone, Debug)]
pub struct WorldTrace {
    events: Vec<Event>,
}

impl WorldTrace {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn spawn(&mut self, frame: ControlFrame, entity: Entity) {
        let timestamp = SystemTime::now();

        self.events.push(Event {
            timestamp,
            frame,
            body: EventBody::Spawn(entity),
        });
    }

    pub fn despawn(&mut self, frame: ControlFrame, id: EntityId) {
        let timestamp = SystemTime::now();

        self.events.push(Event {
            timestamp,
            frame,
            body: EventBody::Despawn(id),
        });
    }

    pub fn set_translation(&mut self, frame: ControlFrame, id: EntityId, translation: Vec3) {
        let timestamp = SystemTime::now();

        self.events.push(Event {
            timestamp,
            frame,
            body: EventBody::Translate(id, translation),
        });
    }

    pub fn set_rotation(&mut self, frame: ControlFrame, id: EntityId, rotation: Quat) {
        let timestamp = SystemTime::now();

        self.events.push(Event {
            timestamp,
            frame,
            body: EventBody::Rotate(id, rotation),
        });
    }
}

#[derive(Clone, Debug)]
struct Event {
    timestamp: SystemTime,
    frame: ControlFrame,
    body: EventBody,
}

#[derive(Clone, Debug)]
enum EventBody {
    Spawn(Entity),
    Despawn(EntityId),
    Translate(EntityId, Vec3),
    Rotate(EntityId, Quat),
}
