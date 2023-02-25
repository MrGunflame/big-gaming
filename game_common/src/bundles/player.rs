use bevy_ecs::prelude::Bundle;
use glam::Vec3;

use crate::components::actor::ActorFigure;
use crate::components::player::HostPlayer;
use crate::world::source::StreamingSource;

use super::ActorBundle;

#[derive(Bundle)]
pub struct HostPlayerBundle {
    #[bundle]
    pub actor: ActorBundle,
    pub host_player: HostPlayer,
    pub streaming_source: StreamingSource,
    pub actor_figure: ActorFigure,
}

impl HostPlayerBundle {
    pub fn new() -> Self {
        Self {
            actor: ActorBundle::default(),
            host_player: HostPlayer,
            streaming_source: StreamingSource::new(),
            actor_figure: ActorFigure {
                eyes: Vec3::new(0.0, 1.5, 0.0),
            },
        }
    }
}
