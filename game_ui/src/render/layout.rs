use std::collections::HashMap;

use bevy_ecs::prelude::Component;
use glam::Vec2;

use super::image::Image;
use super::style::Direction;
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

    fn bounds(&self) -> Bounds {
        match &self.body {
            ElementBody::Container() => todo!(),
            ElementBody::Image(elem) => elem.bounds(),
            ElementBody::Text(elem) => elem.bounds(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ElementBody {
    Container(),
    Image(Image),
    Text(Text),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Bounds {
    pub min: Option<Vec2>,
    pub max: Option<Vec2>,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            min: Some(Vec2::splat(0.0)),
            max: Some(Vec2::splat(f32::INFINITY)),
        }
    }
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
    /// Root elements
    root: Vec<usize>,
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
            root: vec![],
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
        } else {
            self.root.push(index);
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
        let mut allocated = Vec2::splat(0.0);
        let direction = Direction::Row;
        for key in &self.root {
            let elem = &self.elems[*key];

            let min_width = elem.bounds().min.unwrap_or_default().y;
            let min_height = elem.bounds().min.unwrap_or_default().x;

            // Create the layout based on the minimal size.
            let layout = &mut self.layouts[*key];
            layout.position = Vec2::new(allocated.x, 0.0);
            layout.width = min_width;
            layout.height = min_height;

            match direction {
                Direction::Row => {
                    allocated.x += min_width;
                }
                Direction::Column => {
                    allocated.y += min_height;
                }
            }

            // width += min_width;
        }

        let mut next_position = Vec2::splat(0.0);
        let size_per_elem = size_per_element(self.size, self.root.len() as u32, direction);
        for key in &self.root {
            // Every elements gets `size_per_elem` or `max`, whichever is lower.
            let elem = &self.elems[*key];
            let layout = &mut self.layouts[*key];

            let max = elem.bounds().max.unwrap_or(Vec2::splat(f32::INFINITY));

            layout.position = next_position;
            match direction {
                Direction::Row => {
                    layout.width = f32::min(size_per_elem.x, max.x);
                    layout.height = f32::min(size_per_elem.y, max.y);

                    next_position.y += layout.height;
                }
                Direction::Column => {
                    layout.width = f32::min(size_per_elem.x, max.x);
                    layout.height = f32::min(size_per_elem.y, max.y);

                    next_position.x += layout.width;
                }
            }
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

fn size_per_element(space: Vec2, num_elems: u32, direction: Direction) -> Vec2 {
    match direction {
        Direction::Row => {
            let width = space.x;
            let height = space.y / num_elems as f32;

            Vec2::new(width, height)
        }
        Direction::Column => {
            let width = space.x / num_elems as f32;
            let height = space.y;

            Vec2::new(width, height)
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use crate::render::style::Direction;

    use super::size_per_element;

    #[test]
    fn size_per_element_row() {
        let space = Vec2::splat(1000.0);
        let num_elems = 5;
        let direction = Direction::Row;

        let output = size_per_element(space, num_elems, direction);

        assert_eq!(output.x, 1000.0);
        assert_eq!(output.y, 200.0);
    }

    #[test]
    fn size_per_element_column() {
        let space = Vec2::splat(1000.0);
        let num_elems = 5;
        let direction = Direction::Column;

        let output = size_per_element(space, num_elems, direction);

        assert_eq!(output.x, 200.0);
        assert_eq!(output.y, 1000.0);
    }
}
