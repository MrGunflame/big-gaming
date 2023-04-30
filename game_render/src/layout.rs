use glam::Vec2;

/// The global wrapper for all UI elements.
#[derive(Clone, Debug)]
pub struct Frame {
    nodes: Vec<Rect>,
}

impl Frame {
    pub fn draw(&self, ctx: &mut DrawContext) {
        for node in &self.nodes {
            node.draw(ctx);
        }
    }
}

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
        // Map (0.0, 0.0) to (-1.0, 1.0).
        let start_x = self.position.x - 1.0;
        let start_y = self.position.y + 1.0;
        let origin = Vec2::new(start_x, start_y);

        let vertices = [
            [origin.x, origin.y, 0.0],
            [origin.x, origin.y - self.height, 0.0],
            [origin.x + self.width, origin.y - self.height, 0.0],
            [origin.x + self.width, origin.y, 0.0],
        ];
        let indicies = [0, 1, 2, 3, 0, 2];

        ctx.vertices(&vertices);
        ctx.indices(&indicies);
    }
}

pub struct DrawContext {
    size: Vec2,
    pub(crate) vertex: Vec<Vertex>,
    pub(crate) indices: Vec<u32>,
}

impl DrawContext {
    pub(crate) fn new(size: Vec2) -> Self {
        Self {
            size,
            vertex: vec![],
            indices: vec![],
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
        self.indices.extend(indices);
    }
}

pub enum Node {
    Rect(Rect),
}
