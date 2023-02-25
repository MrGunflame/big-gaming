use std::sync::Arc;

use bevy_ecs::component::Component;
use bevy_ecs::system::Resource;

#[derive(Clone, Debug, Component)]
pub enum Scene {
    Loading,
    MainMenu,
    World,
    ServerConnect { addr: String },
    ServerError(ServerError),
}

#[derive(Clone, Debug, Component)]
pub enum ServerError {
    Connection(Arc<dyn std::error::Error + Send + Sync + 'static>),
}

impl<T> From<T> for ServerError
where
    T: std::error::Error + Send + Sync + 'static,
{
    fn from(value: T) -> Self {
        let err = Arc::new(value);
        Self::Connection(err)
    }
}

#[derive(Clone, Debug, Resource)]
pub struct SceneTransition {
    pub from: Scene,
    pub to: Scene,
}
