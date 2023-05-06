use std::collections::HashMap;

use bevy_ecs::prelude::Component;
use glam::Vec2;

use super::image::Image;
use super::text::Text;
use super::BuildPrimitiveElement;

#[derive(Clone, Debug)]
pub struct Element {
    pub bounds: Bounds,
    pub body: ElementBody,
}

impl BuildPrimitiveElement for Element {
    fn build(
        &self,
        layout: super::Rect,
        pipeline: &super::UiPipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: Vec2,
    ) -> super::PrimitiveElement {
        match &self.body {
            ElementBody::Container() => todo!(),
            ElementBody::Image(elem) => elem.build(layout, pipeline, device, queue, size),
            ElementBody::Text(elem) => elem.build(layout, pipeline, device, queue, size),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ElementBody {
    Container(),
    Image(Image),
    Text(Text),
}

#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct Bounds {
    pub min: Option<Vec2>,
    pub max: Option<Vec2>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(usize);

#[derive(Clone, Debug, Component)]
pub struct LayoutTree {
    elems: Vec<Element>,
    layouts: Vec<Layout>,
    size: Vec2,
    changed: bool,

    children: HashMap<usize, Vec<usize>>,
    parents: HashMap<usize, usize>,
}

impl LayoutTree {
    pub fn new() -> Self {
        Self {
            elems: vec![],
            layouts: vec![],
            size: Vec2::splat(0.0),
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
        let index = self.elems.len();

        self.elems.push(elem);
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
        for (elem, layout) in self.elems.iter().zip(self.layouts.iter_mut()) {
            layout.position.y = y;
            y += elem.bounds.min.map(|v| v.y).unwrap_or(0.0);
        }
    }

    /// Computes the minimal bounds from the botton up.
    fn element_bounds(&mut self) {
        // Start with the leaf elements, then go bottom up.
        let mut children = self.children.clone();

        while children.len() > 0 {
            // Lay out all leaf nodes.
            for (index, _) in children.clone().iter().filter(|(_, c)| c.len() == 0) {
                let elem = &self.elems[*index];

                let dimensions = if let Some(childs) = self.children.get(index) {
                    if childs.is_empty() {
                        elem.bounds.min.unwrap_or_default()
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
                    elem.bounds.min.unwrap_or_default()
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

    pub fn elements(&self) -> Elements<'_> {
        Elements { inner: &self.elems }
    }

    pub fn layouts(&self) -> Layouts<'_> {
        Layouts {
            inner: &self.layouts,
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = Key> {
        (0..self.elems.len()).map(Key)
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

#[derive(Copy, Clone, Debug)]
pub struct Layout {
    pub position: Vec2,
    pub width: f32,
    pub height: f32,
}
