use game_window::windows::WindowId;
use wgpu::{CommandEncoder, Device, TextureFormat, TextureView};

use crate::forward::ForwardPipeline;
use crate::post_process::PostProcessPipeline;
use crate::surface::SurfaceData;

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
    pub window: WindowId,
    pub encoder: &'a mut CommandEncoder,
    pub target: &'a TextureView,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub device: &'a Device,
    pub surface: &'a SurfaceData,
}
