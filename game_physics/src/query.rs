use game_common::entity::EntityId;

#[derive(Clone, Debug, Default)]
pub struct QueryFilter {
    pub exclude_entities: Vec<EntityId>,
}
