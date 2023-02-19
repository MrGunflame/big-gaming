use bevy_ecs::entity::Entity;
use game_common::net::ServerEntity;
use std::collections::HashMap;

pub struct Snapshot {
    entities: HashMap<Entity, ServerEntity>,
}
