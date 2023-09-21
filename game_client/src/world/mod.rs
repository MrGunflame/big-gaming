use std::net::ToSocketAddrs;

use game_common::components::transform::Transform;
use game_core::counter::Interval;
use game_core::time::Time;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::Renderer;
use game_window::windows::WindowId;

use crate::config::Config;
use crate::net::world::CommandBuffer;
use crate::net::ServerConnection;

#[derive(Debug)]
pub struct GameWorldState {
    pub conn: ServerConnection<Interval>,
}

impl GameWorldState {
    pub fn new(config: &Config, addr: impl ToSocketAddrs) -> Self {
        let mut conn = ServerConnection::new(config);
        conn.connect(addr);

        Self { conn }
    }

    pub fn update(&mut self, renderer: &mut Renderer, window: WindowId, time: &Time) {
        let camera = Camera {
            transform: Transform::default(),
            projection: Projection::default(),
            target: RenderTarget::Window(window),
        };

        renderer.entities.cameras.insert(camera);

        let mut buf = CommandBuffer::new();
        self.conn.update(time, &mut buf);
    }
}
