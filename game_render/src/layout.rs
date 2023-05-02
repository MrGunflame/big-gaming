use std::collections::HashMap;

use glam::Vec2;
use image::ImageBuffer;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue};

#[derive(Clone, Debug)]
pub enum Element {
    Container(Container),
    Text(Text),
    Image(Image),
}

impl Element {
    pub fn dimensions(&self) -> Vec2 {
        match self {
            Self::Container(elem) => elem.dimensions(),
            Self::Text(elem) => elem.dimensions(),
            Self::Image(elem) => elem.dimensions(),
        }
    }
}

impl BuildPrimitiveElement for Element {
    fn build(
        &self,
        position: Vec2,
        pipeline: &crate::ui::UiPipeline,
        device: &Device,
        queue: &Queue,
        size: Vec2,
    ) -> crate::ui::PrimitiveElement {
        match self {
            Self::Container(elem) => elem.build(position, pipeline, device, queue, size),
            Self::Text(elem) => elem.build(position, pipeline, device, queue, size),
            Self::Image(elem) => elem.build(position, pipeline, device, queue, size),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Container {
    pub position: Vec2,
}

impl Container {
    pub fn dimensions(&self) -> Vec2 {
        Vec2::new(100.0, 100.0)
    }
}

impl BuildPrimitiveElement for Container {
    fn build(
        &self,
        position: Vec2,
        pipeline: &crate::ui::UiPipeline,
        device: &Device,
        queue: &Queue,
        size: Vec2,
    ) -> crate::ui::PrimitiveElement {
        let mut image = ImageBuffer::new(100, 100);
        debug_border(&mut image);

        let start = crate::layout::remap(position, size);
        let end = crate::layout::remap(position + Vec2::new(100.0, 100.0), size);

        PrimitiveElement::new(
            pipeline,
            device,
            queue,
            start,
            end,
            &image,
            [0.0, 1.0, 0.0, 1.0],
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(usize);

/// The global wrapper for all UI elements.
#[derive(Debug)]
pub struct Frame {
    nodes: Vec<Element>,
    layouts: Vec<Layout>,
    size: Vec2,
    changed: bool,

    children: HashMap<usize, Vec<usize>>,
    parents: HashMap<usize, usize>,
}

impl Frame {
    pub fn new(size: Vec2) -> Self {
        Self {
            nodes: vec![],
            layouts: vec![],
            size,
            changed: false,
            children: HashMap::new(),
            parents: HashMap::new(),
        }
    }

    pub fn resize(&mut self, size: Vec2) {
        self.size = size;
        self.changed = true;
    }

    pub fn push(&mut self, elem: Element) {
        self.nodes.push(elem);
        self.layouts.push(Layout {
            position: Vec2::splat(0.0),
        });
        self.changed = true;
    }

    pub fn unchanged(&mut self) {
        self.changed = false;
    }

    pub fn is_changed(&self) -> bool {
        self.changed
    }

    pub fn calculate_layout(&mut self) {
        let mut y = 0.0;
        for (elem, layout) in self.nodes.iter().zip(self.layouts.iter_mut()) {
            layout.position.y = y;
            y += elem.dimensions().y;
        }
    }

    // pub fn draw(&mut self, ctx: &mut DrawContext) {
    //     let size = ctx.size();

    //     let mut width = 0.0;
    //     for node in &self.nodes {
    //         width += node.width;
    //     }

    //     // Space between
    //     let padding = (size.x - width) / (self.nodes.len() as f32 - 1.0);

    //     let diff = self.nodes[0].width;
    //     for node in self.nodes.iter_mut().skip(1) {
    //         node.position.x += diff;
    //         node.position.x += padding;
    //     }

    //     for node in self.nodes.iter() {
    //         node.draw(ctx);
    //         ctx.update_vertex_counter();
    //     }
    // }

    /// Returns an iterator over all elements in the `Frame`.
    pub fn elements(&self) -> Elements<'_> {
        Elements { inner: &self.nodes }
    }

    pub fn layouts(&self) -> Layouts<'_> {
        Layouts {
            inner: &self.layouts,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Elements<'a> {
    inner: &'a [Element],
}

impl<'a> Iterator for Elements<'a> {
    type Item = &'a Element;

    fn next(&mut self) -> Option<Self::Item> {
        let (elem, rem) = self.inner.split_first()?;
        self.inner = rem;
        Some(elem)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a> ExactSizeIterator for Elements<'a> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

#[derive(Clone, Debug)]
pub struct Layouts<'a> {
    inner: &'a [Layout],
}

impl<'a> Iterator for Layouts<'a> {
    type Item = &'a Layout;

    fn next(&mut self) -> Option<Self::Item> {
        let (elem, rem) = self.inner.split_first()?;
        self.inner = rem;
        Some(elem)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a> ExactSizeIterator for Layouts<'a> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

use crate::image::{debug_border, Image};
use crate::text::Text;
use crate::ui::{BuildPrimitiveElement, PrimitiveElement};
use crate::Vertex;

#[derive(Copy, Clone, Debug)]
pub struct Layout {
    pub position: Vec2,
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
}

impl<'a> DrawContext<'a> {
    pub(crate) fn new(
        size: Vec2,
        device: &'a mut Device,
        queue: &'a Queue,
        bind_groups: &'a mut Vec<BindGroup>,
    ) -> Self {
        Self {
            size,
            vertex: vec![],
            indices: vec![],
            vertices: 0,
            device,
            queue,
            bind_groups,
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

#[derive(Clone, Debug)]
pub struct Style {
    pub padding: f32,
    pub margin: f32,
}

pub enum Position {
    Auto,
    Relative,
    Absolute,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Size {
    Pixel(f32),
    Percent(f32),
}

/// Remap from absolute screen space to OpenGL vertex translations.
pub fn remap(input: Vec2, size: Vec2) -> Vec2 {
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
