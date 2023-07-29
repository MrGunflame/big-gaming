use bevy_ecs::prelude::Entity;
use bevy_ecs::system::Resource;
use bevy_ecs::world::World;
use wgpu::{CommandEncoder, TextureView};

#[derive(Default, Resource)]
pub struct RenderGraph {
    pub(crate) nodes: Vec<Box<dyn Node>>,
}

impl RenderGraph {
    pub fn push<T: Node>(&mut self, node: T) {
        self.nodes.push(Box::new(node));
    }
}

pub trait Node: Send + Sync + 'static {
    fn update(&mut self, world: &mut World);

    fn render(&self, world: &World, ctx: &mut RenderContext<'_>);
}

pub struct RenderContext<'a> {
    pub window: Entity,
    pub encoder: &'a mut CommandEncoder,
    pub target: &'a TextureView,
    pub width: u32,
    pub height: u32,
}
