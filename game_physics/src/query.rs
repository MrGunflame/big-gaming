use game_common::entity::EntityId;

pub struct QueryFilter {
    pub exclude_entities: Vec<EntityId>,
}
