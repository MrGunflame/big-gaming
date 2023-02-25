use bevy_ecs::component::Component;
use bevy_ecs::system::Resource;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Component)]
pub enum Scene {
    Loading,
    MainMenu,
    World,
    ServerConnect { addr: String },
}

#[derive(Clone, Debug, Resource)]
pub struct SceneTransition {
    pub from: Scene,
    pub to: Scene,
}
