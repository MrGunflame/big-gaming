use bevy_ecs::system::Resource;
use bevy_ecs::world::World;

use crate::ui::RenderContext;

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
