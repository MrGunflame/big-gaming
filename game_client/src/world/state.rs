use game_common::world::World;

#[derive(Clone, Debug, Default)]
pub struct WorldState {
    pub world: World,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            world: World::new(),
        }
    }
}
