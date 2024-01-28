use glam::UVec2;
use wgpu::{CommandEncoder, Device, Queue, TextureFormat, TextureView};

use crate::camera::RenderTarget;
use crate::mipmap::MipMapGenerator;

#[derive(Default)]
pub struct RenderGraph {
    pub(crate) nodes: Vec<Box<dyn Node>>,
}

impl RenderGraph {
    pub fn push<T: Node>(&mut self, node: T) {
        self.nodes.push(Box::new(node));
    }
}

pub trait Node: Send + Sync + 'static {
    fn render(&self, ctx: &mut RenderContext<'_>);
}

pub struct RenderContext<'a> {
    pub render_target: RenderTarget,
    pub encoder: &'a mut CommandEncoder,
    pub target: &'a TextureView,
    pub size: UVec2,
    pub format: TextureFormat,
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub mipmap: &'a mut MipMapGenerator,
}
