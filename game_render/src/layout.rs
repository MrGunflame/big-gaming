use glam::Vec2;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue};

/// The global wrapper for all UI elements.
#[derive(Clone, Debug)]
pub struct Frame {
    nodes: Vec<Rect>,
}

impl Frame {
    pub fn new() -> Self {
        Self { nodes: vec![] }
    }

    pub fn push(&mut self, node: Rect) {
        self.nodes.push(node);
    }

    pub fn draw(&mut self, ctx: &mut DrawContext) {
        let size = ctx.size();

        let mut width = 0.0;
        for node in &self.nodes {
            width += node.width;
        }

        // Space between
        let padding = (size.x - width) / (self.nodes.len() as f32 - 1.0);

        let diff = self.nodes[0].width;
        for node in self.nodes.iter_mut().skip(1) {
            node.position.x += diff;
            node.position.x += padding;
        }

        for node in self.nodes.iter() {
            node.draw(ctx);
            ctx.update_vertex_counter();
        }
    }
}

use crate::text::TextPipeline;
use crate::Vertex;

pub struct Layout {
    children: Vec<Node>,
}

impl Layout {}

#[derive(Clone, Debug)]
pub struct Style {
    pub padding: f32,
}

pub trait Widget {
    fn draw(&self, ctx: &mut DrawContext);
}

#[derive(Clone, Debug)]
pub struct Rect {
    pub position: Vec2,
    pub width: f32,
    pub height: f32,
}

impl Widget for Rect {
    fn draw(&self, ctx: &mut DrawContext) {
        let start = remap(self.position, ctx.size());
        let end = remap(
            Vec2::new(self.position.x + self.width, self.position.y + self.height),
            ctx.size(),
        );

        let vertices = [
            [start.x, start.y, 0.0],
            [start.x, end.y, 0.0],
            [end.x, end.y, 0.0],
            [end.x, start.y, 0.0],
        ];
        let indicies = [0, 1, 2, 3, 0, 2];

        ctx.vertices(&vertices);
        ctx.indices(&indicies);
    }
}

pub struct DrawContext<'a> {
    size: Vec2,
    pub(crate) vertex: Vec<Vertex>,
    pub(crate) indices: Vec<u32>,
    vertices: u32,
    device: &'a mut Device,
    queue: &'a Queue,
    pub(crate) bind_groups: &'a mut Vec<BindGroup>,
    pub(crate) text_pipeline: &'a mut TextPipeline,
}

impl<'a> DrawContext<'a> {
    pub(crate) fn new(
        size: Vec2,
        device: &'a mut Device,
        queue: &'a Queue,
        bind_groups: &'a mut Vec<BindGroup>,
        text_pipeline: &'a mut TextPipeline,
    ) -> Self {
        Self {
            size,
            vertex: vec![],
            indices: vec![],
            vertices: 0,
            device,
            queue,
            bind_groups,
            text_pipeline,
        }
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn push(&mut self, vertex: Vertex) {
        self.vertex.push(vertex);
    }

    pub fn vertices(&mut self, vertices: &[[f32; 3]]) {
        self.vertex.extend(vertices.iter().map(|v| Vertex {
            position: *v,
            color: [1.0, 0.0, 0.0],
        }))
    }

    pub fn indices(&mut self, indices: &[u32]) {
        for mut index in indices.iter().copied() {
            index += self.vertices;
            self.indices.push(index);
        }
    }

    fn update_vertex_counter(&mut self) {
        self.vertices = self.vertex.len() as u32;
    }

    pub fn device(&'a self) -> &'a Device {
        &self.device
    }

    pub fn queue(&'a self) -> &'a Queue {
        &self.queue
    }
}

pub enum Node {
    Rect(Rect),
}

/// Remap from absolute screen space to OpenGL vertex translations.
fn remap(input: Vec2, size: Vec2) -> Vec2 {
    let x = lerp(-1.0, 1.0, input.x / size.x);
    let y = lerp(1.0, -1.0, input.y / size.y);
    Vec2::new(x, y)
}

fn lerp(lhs: f32, rhs: f32, s: f32) -> f32 {
    lhs + ((rhs - lhs) * s)
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    #[test]
    fn remap_baseline() {
        let input = Vec2::new(0.0, 0.0);
        let size = Vec2::new(1000.0, 1000.0);

        assert_eq!(super::remap(input, size), Vec2::new(-1.0, 1.0));
    }

    #[test]
    fn remap_center() {
        let input = Vec2::new(500.0, 500.0);
        let size = Vec2::splat(1000.0);

        assert_eq!(super::remap(input, size), Vec2::new(0.0, 0.0));
    }

    #[test]
    fn remap() {
        let input = Vec2::new(400.0, 600.0);
        let size = Vec2::splat(1000.0);

        // FP inacuraccy
        assert_eq!(super::remap(input, size), Vec2::new(-0.2, -0.2));
    }
}

pub trait AsBindGroup {
    const NAME: &'static str;

    fn as_bind_group(&self);

    fn bind_group_layout(device: &Device) -> BindGroupLayout;
}
