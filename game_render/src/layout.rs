use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
        layout: Layout,
        pipeline: &crate::ui::UiPipeline,
        device: &Device,
        queue: &Queue,
        size: Vec2,
    ) -> crate::ui::PrimitiveElement {
        match self {
            Self::Container(elem) => elem.build(layout, pipeline, device, queue, size),
            Self::Text(elem) => elem.build(layout, pipeline, device, queue, size),
            Self::Image(elem) => elem.build(layout, pipeline, device, queue, size),
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
        layout: Layout,
        pipeline: &crate::ui::UiPipeline,
        device: &Device,
        queue: &Queue,
        size: Vec2,
    ) -> crate::ui::PrimitiveElement {
        let mut image = ImageBuffer::new(100, 100);
        debug_border(&mut image);

        let start = crate::layout::remap(layout.position, size);
        let end = crate::layout::remap(
            layout.position + Vec2::new(layout.width, layout.height),
            size,
        );

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

    pub fn push(&mut self, parent: Option<Key>, elem: Element) -> Key {
        let index = self.nodes.len();

        self.nodes.push(elem);
        self.layouts.push(Layout {
            position: Vec2::splat(0.0),
            height: 0.0,
            width: 0.0,
        });

        self.children.insert(index, vec![]);

        if let Some(Key(parent)) = parent {
            self.children.get_mut(&parent).unwrap().push(index);
            self.parents.insert(index, parent);
        }

        self.changed = true;

        Key(index)
    }

    pub fn unchanged(&mut self) {
        self.changed = false;
    }

    pub fn is_changed(&self) -> bool {
        self.changed
    }

    pub fn compute_layout(&mut self) {
        self.element_bounds();
        // self.element_positions();

        let mut y = 0.0;
        for (elem, layout) in self.nodes.iter().zip(self.layouts.iter_mut()) {
            layout.position.y = y;
            y += elem.dimensions().y;
        }
    }

    /// Computes the minimal bounds from the botton up.
    fn element_bounds(&mut self) {
        // Start with the leaf elements, then go bottom up.
        let mut children = self.children.clone();

        while children.len() > 0 {
            // Lay out all leaf nodes.
            for (index, _) in children.clone().iter().filter(|(_, c)| c.len() == 0) {
                let elem = &self.nodes[*index];

                let dimensions = if let Some(childs) = self.children.get(index) {
                    if childs.is_empty() {
                        elem.dimensions()
                    } else {
                        // The dimensions of the element with children are the sum of
                        // the dimensions of all children in one direction, and the maximum
                        // in the other.
                        let mut width = 0.0;
                        let mut height = 0.0;

                        for child in childs {
                            let layout = &self.layouts[*child];
                            width += layout.width;
                            height += layout.height;
                        }

                        Vec2::new(width, height)
                    }
                } else {
                    // Elements without children, usually leaf nodes.
                    elem.dimensions()
                };

                let layout = &mut self.layouts[*index];
                layout.width = dimensions.x;
                layout.height = dimensions.y;

                if let Some(parent) = self.parents.get(index) {
                    let (idx, _) = children
                        .get_mut(parent)
                        .unwrap()
                        .iter()
                        .enumerate()
                        .find(|(_, child)| *child == index)
                        .unwrap();

                    children.get_mut(parent).unwrap().remove(idx);
                }

                children.remove(&index);
            }
        }
    }

    fn element_positions(&mut self) {
        for (index, childs) in &self.children {
            // Get parent position.
            let layout = &self.layouts[*index];

            let mut width = layout.position.x;
            let mut height = layout.position.y;

            for child in childs {
                let elem = &mut self.layouts[*child];
                elem.position = Vec2::new(width, height);
                width += elem.width;
                height += elem.height;
            }
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

#[derive(Copy, Clone, Debug)]
pub struct Layout {
    pub position: Vec2,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug)]
pub struct Rect {
    pub position: Vec2,
    pub width: f32,
    pub height: f32,
}

pub enum Node {
    Rect(Rect),
}

#[derive(Clone, Debug)]
pub struct Style {
    pub padding: f32,
    pub margin: f32,
}

pub enum Direction {
    Row,
    Column,
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
