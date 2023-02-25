use bevy_ecs::component::Component;
use bevy_ecs::system::Resource;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Component)]
pub enum Scene {
    Loading,
    MainMenu,
    World,
}

#[derive(Clone, Debug, Resource)]
pub struct SceneTransition {
    pub from: Scene,
    pub to: Scene,
}
