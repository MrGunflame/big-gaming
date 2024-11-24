use game_common::entity::EntityId;

#[derive(Clone, Debug, Default)]
pub struct QueryFilter {
    pub exclude_entities: Vec<EntityId>,
}

#[derive(Copy, Clone, Debug)]
pub struct QueryHit {
    /// The entity that was hit.
    pub entity: EntityId,
    /// The time-of-impact (distance along the path) at which the entity was hit.
    pub toi: f32,
}
