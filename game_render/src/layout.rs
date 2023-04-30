use glam::Vec2;

use crate::Vertex;

pub struct Layout {
    children: Vec<Node>,
}

impl Layout {}

#[derive(Clone, Debug)]
pub struct Node {
    width: f32,
    height: f32,
}

#[derive(Clone, Debug)]
pub struct Style {
    pub padding: f32,
}

pub trait Widget {
    fn draw(&self, ctx: &mut DrawContext);
}

pub struct Rect {
    pub position: Vec2,
    pub width: f32,
    pub height: f32,
}

impl Widget for Rect {
    fn draw(&self, ctx: &mut DrawContext) {
        let vertices = [[-1.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 0.0]];

        let vertices = [
            Vertex {
                position: [-1.0, 1.0, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [-1.0, 0.0, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [0.0, 0.0, 0.0],
                color: [0.0, 0.0, 1.0],
            },
            // 2
            Vertex {
                position: [0.0, 1.0, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [-1.0, 1.0, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [0.0, 0.0, 0.0],
                color: [0.0, 0.0, 1.0],
            },
        ];

        ctx.push(vertices[0]);
        ctx.push(vertices[1]);
        ctx.push(vertices[2]);
        ctx.push(vertices[3]);
        ctx.push(vertices[4]);
        ctx.push(vertices[5]);
    }
}

pub struct DrawContext {
    size: Vec2,
    pub(crate) vertex: Vec<Vertex>,
}

impl DrawContext {
    pub(crate) fn new(size: Vec2) -> Self {
        Self {
            size,
            vertex: vec![],
        }
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn push(&mut self, vertex: Vertex) {
        self.vertex.push(vertex);
    }
}
