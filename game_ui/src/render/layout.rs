use std::collections::HashMap;

use bevy_ecs::prelude::Component;
use glam::Vec2;
use slotmap::{DefaultKey, SlotMap};

use super::container::Container;
use super::image::Image;
use super::style::{Bounds, Direction, Position, Style};
use super::text::Text;
use super::BuildPrimitiveElement;

#[derive(Clone, Debug)]
pub struct Element {
    pub body: ElementBody,
    pub style: Style,
}

impl BuildPrimitiveElement for Element {
    fn build(
        &self,
        style: &Style,
        layout: super::Rect,
        pipeline: &super::UiPipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: Vec2,
    ) -> Option<super::PrimitiveElement> {
        match &self.body {
            ElementBody::Container() => {
                Container.build(style, layout, pipeline, device, queue, size)
            }
            ElementBody::Image(elem) => elem.build(style, layout, pipeline, device, queue, size),
            ElementBody::Text(elem) => elem.build(style, layout, pipeline, device, queue, size),
        }
    }

    fn bounds(&self) -> ComputedBounds {
        match &self.body {
            ElementBody::Container() => ComputedBounds::default(),
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(DefaultKey);

#[derive(Clone, Debug, Component)]
pub struct LayoutTree {
    elems: SlotMap<DefaultKey, Element>,
    layouts: HashMap<DefaultKey, Layout>,
    size: Vec2,
    changed: bool,

    // parent => vec![child]
    children: HashMap<DefaultKey, Vec<DefaultKey>>,
    // child => parent
    parents: HashMap<DefaultKey, DefaultKey>,
    /// Root elements
    root: Vec<DefaultKey>,
}

impl LayoutTree {
    pub fn new() -> Self {
        Self {
            elems: SlotMap::new(),
            layouts: HashMap::new(),
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
        let layout = Layout {
            position: Vec2::splat(0.0),
            height: 0.0,
            width: 0.0,
            style: ComputedStyle {
                bounds: ComputedBounds::new(elem.style.bounds, self.size),
            },
        };

        let key = self.elems.insert(elem);
        self.layouts.insert(key, layout);

        self.children.insert(key, vec![]);

        if let Some(Key(parent)) = parent {
            self.children.get_mut(&parent).unwrap().push(key);
            self.parents.insert(key, parent);
        } else {
            self.root.push(key);
        }

        self.changed = true;

        Key(key)
    }

    pub fn remove(&mut self, key: Key) {
        self.elems.remove(key.0);
        self.layouts.remove(&key.0);

        self.root.retain(|k| *k != key.0);

        if let Some(children) = self.children.remove(&key.0) {
            for c in children {
                self.remove(Key(c));
            }
        }

        if let Some(parent) = self.parents.remove(&key.0) {
            if let Some(children) = self.children.get_mut(&parent) {
                children.retain(|k| *k != key.0);
            }
        }
    }

    pub fn unchanged(&mut self) {
        self.changed = false;
    }

    pub fn is_changed(&self) -> bool {
        self.changed
    }

    pub fn compute_layout(&mut self) {
        self.computed_sizes();

        let mut bounds = ComputedBounds::default();
        for key in &self.root {
            let child_bounds = self.compute_bounds(*key);

            // Root elements always flow into Row direction.
            bounds.min.x = f32::max(bounds.min.x, child_bounds.min.x);
            bounds.min.y += bounds.min.y;

            bounds.max.x = f32::max(bounds.max.x, child_bounds.max.x);
            bounds.max.y += bounds.max.y;

            // let elem = &self.elems[*key];

            // let min_width = elem.bounds().min.unwrap_or_default().y;
            // let min_height = elem.bounds().min.unwrap_or_default().x;

            // Create the layout based on the minimal size.
            // let layout = &mut self.layouts[*key];
            // layout.position = Vec2::new(allocated.x, 0.0);
            // layout.width = min_width;
            // layout.height = min_height;

            // match direction {
            //     Direction::Row => {
            //         allocated.x += min_width;
            //     }
            //     Direction::Column => {
            //         allocated.y += min_height;
            //     }
            // }

            // width += min_width;
        }

        let mut next_position = Vec2::splat(0.0);
        let size_per_elem = size_per_element(self.size, self.root.len() as u32, Direction::Row);
        // FIXME: No need to clone, layout_element doesn't touch self.root.
        for key in &self.root.clone() {
            let bounds = self.compute_bounds(*key);

            let layout = self.layouts.get_mut(key).unwrap();

            // Every elements gets `size_per_elem` or `max`, whichever is lower.
            layout.position = next_position;
            layout.width = f32::clamp(size_per_elem.x, bounds.min.x, bounds.max.x);
            layout.height = f32::clamp(size_per_elem.y, bounds.min.y, bounds.max.y);

            next_position.y += layout.height;

            self.layout_element(*key);
        }

        // for key in &self.root {
        //     let elem = &self.elems[*key];
        //     let layout = &mut self.layouts[*key];

        //     let max = elem.bounds().max.unwrap_or(Vec2::splat(f32::INFINITY));
        //     layout.width = f32::min(size_per_elem.x, max.x);
        //     layout.height = f32::min(size_per_elem.y, max.y);

        //     layout.position = next_position;
        //     match direction {
        //         Direction::Row => next_position.y += layout.height,
        //         Direction::Column => next_position.x += layout.width,
        //     }
        // }
    }

    fn compute_bounds(&self, key: DefaultKey) -> ComputedBounds {
        let elem = &self.elems[key];

        match &elem.body {
            ElementBody::Container() => {
                // Infer the bounds from the children elements.
                if let Some(children) = self.children.get(&key) {
                    let mut bounds = ComputedBounds::ZERO;
                    for key in children {
                        // Elements with absolute position are excluded.
                        let child = &self.elems[*key];
                        if child.style.position.is_absolute() {
                            continue;
                        }

                        let child_bounds = self.compute_bounds(*key);

                        let min = child_bounds.min;
                        let max = child_bounds.max;

                        match elem.style.direction {
                            Direction::Row => {
                                bounds.min.y += min.y;
                                bounds.min.x = f32::max(bounds.min.x, min.x);

                                bounds.max.y += max.y;
                                bounds.max.x = f32::max(bounds.max.x, max.x);
                            }
                            Direction::Column => {
                                bounds.min.y = f32::min(bounds.min.y, min.y);
                                bounds.min.x += min.x;

                                bounds.max.y = f32::max(bounds.max.y, max.y);
                                bounds.max.x += max.x;
                            }
                        }
                    }

                    if elem.style.growth.0.is_some() {
                        bounds.max = Vec2::splat(f32::INFINITY);
                    }

                    bounds
                } else {
                    // If the container can grow, it may take any size.
                    // If the can not grow, it will always have the size zero.
                    if elem.style.growth.0.is_some() {
                        ComputedBounds::default()
                    } else {
                        ComputedBounds::ZERO
                    }
                }
            }
            ElementBody::Image(elem) => elem.bounds(),
            ElementBody::Text(elem) => elem.bounds(),
        }
    }

    fn layout_element(&mut self, key: DefaultKey) {
        let elem = self.elems[key].clone();
        let layout = &self.layouts[&key];

        let start = layout.position;
        let end = Vec2::new(
            layout.position.x + layout.width,
            layout.position.y + layout.height,
        );

        if let Some(children) = self.children.get(&key).cloned() {
            let mut next_position = layout.position;
            let size_per_elem =
                size_per_element(end - start, children.len() as u32, elem.style.direction);

            for child in children {
                let child_style = &self.elems[child].style;

                let bounds = self.compute_bounds(child);
                let layout = self.layouts.get_mut(&child).unwrap();

                match child_style.position {
                    Position::Relative => {
                        layout.position = next_position;
                        layout.width = f32::clamp(size_per_elem.x, bounds.min.x, bounds.max.x);
                        layout.height = f32::clamp(size_per_elem.y, bounds.min.y, bounds.max.y);

                        match elem.style.direction {
                            Direction::Row => next_position.y += layout.height,
                            Direction::Column => next_position.x += layout.width,
                        }
                    }
                    Position::Absolute(pos) => {
                        // Give the absolute element as much space as it wants
                        // as long as it doesn't overflow the viewport.
                        layout.position = pos;
                        layout.width = f32::min(self.size.x - pos.x, bounds.max.x);
                        layout.height = f32::min(self.size.y - pos.y, bounds.max.y);
                    }
                }

                self.layout_element(child);
            }
        }
    }

    fn computed_sizes(&mut self) {
        for ((_, elem), (_, layout)) in self.elems.iter().zip(self.layouts.iter_mut()) {
            layout.style.bounds = ComputedBounds::new(elem.style.bounds, self.size);
        }
    }

    // / Computes the minimal bounds from the botton up.
    // fn element_bounds(&mut self) {
    //     // Start with the leaf elements, then go bottom up.
    //     let mut children = self.children.clone();

    //     while children.len() > 0 {
    //         // Lay out all leaf nodes.
    //         for (index, _) in children.clone().iter().filter(|(_, c)| c.len() == 0) {
    //             let elem = &self.elems[*index];

    //             let dimensions = if let Some(childs) = self.children.get(index) {
    //                 if childs.is_empty() {
    //                     elem.style.bounds.min
    //                 } else {
    //                     // The dimensions of the element with children are the sum of
    //                     // the dimensions of all children in one direction, and the maximum
    //                     // in the other.
    //                     let mut width = 0.0;
    //                     let mut height = 0.0;

    //                     for child in childs {
    //                         let layout = &self.layouts[*child];
    //                         width += layout.width;
    //                         height += layout.height;
    //                     }

    //                     Vec2::new(width, height)
    //                 }
    //             } else {
    //                 // Elements without children, usually leaf nodes.
    //                 elem.style.bounds.min
    //             };

    //             let layout = &mut self.layouts[*index];
    //             layout.width = dimensions.x;
    //             layout.height = dimensions.y;

    //             if let Some(parent) = self.parents.get(index) {
    //                 let (idx, _) = children
    //                     .get_mut(parent)
    //                     .unwrap()
    //                     .iter()
    //                     .enumerate()
    //                     .find(|(_, child)| *child == index)
    //                     .unwrap();

    //                 children.get_mut(parent).unwrap().remove(idx);
    //             }

    //             children.remove(&index);
    //         }
    //     }
    // }

    fn element_positions(&mut self) {
        for (key, childs) in &self.children {
            // Get parent position.
            let layout = &self.layouts[key];

            let mut width = layout.position.x;
            let mut height = layout.position.y;

            for child in childs {
                let elem = self.layouts.get_mut(child).unwrap();
                elem.position = Vec2::new(width, height);
                width += elem.width;
                height += elem.height;
            }
        }
    }

    pub fn elements(&self) -> Elements<'_> {
        Elements {
            iter: self.elems.iter(),
        }
    }

    pub fn layouts(&self) -> Layouts<'_> {
        Layouts {
            keys: self.elems.iter(),
            layouts: &self.layouts,
        }
    }

    pub fn keys<'a>(&'a self) -> impl Iterator<Item = Key> + 'a {
        self.elems.keys().map(|k| Key(k))
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut Element> {
        match self.elems.get_mut(key.0) {
            Some(elem) => {
                self.changed = true;
                Some(elem)
            }
            None => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Elements<'a> {
    iter: slotmap::basic::Iter<'a, DefaultKey, Element>,
}

impl<'a> Iterator for Elements<'a> {
    type Item = &'a Element;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a> ExactSizeIterator for Elements<'a> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

#[derive(Clone, Debug)]
pub struct Layouts<'a> {
    // Order is important.
    keys: slotmap::basic::Iter<'a, DefaultKey, Element>,
    layouts: &'a HashMap<DefaultKey, Layout>,
}

impl<'a> Iterator for Layouts<'a> {
    type Item = &'a Layout;

    fn next(&mut self) -> Option<Self::Item> {
        self.keys.next().map(|(k, _)| self.layouts.get(&k).unwrap())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a> ExactSizeIterator for Layouts<'a> {
    fn len(&self) -> usize {
        self.keys.len()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Layout {
    style: ComputedStyle,

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

#[derive(Copy, Clone, Debug)]
struct ComputedStyle {
    bounds: ComputedBounds,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct ComputedBounds {
    pub(crate) min: Vec2,
    pub(crate) max: Vec2,
}

impl ComputedBounds {
    const ZERO: Self = Self {
        min: Vec2::splat(0.0),
        max: Vec2::splat(0.0),
    };

    fn new(bounds: Bounds, viewport: Vec2) -> Self {
        Self {
            min: Vec2 {
                x: bounds.min.x.to_pixels(viewport),
                y: bounds.min.y.to_pixels(viewport),
            },
            max: Vec2 {
                x: bounds.max.x.to_pixels(viewport),
                y: bounds.max.y.to_pixels(viewport),
            },
        }
    }
}

impl Default for ComputedBounds {
    fn default() -> Self {
        Self {
            min: Vec2::splat(0.0),
            max: Vec2::splat(f32::INFINITY),
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use crate::render::layout::ComputedBounds;
    use crate::render::style::{Direction, Growth, Position, Style};
    use crate::render::{BuildPrimitiveElement, Text};

    use super::{size_per_element, Element, ElementBody, LayoutTree};

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

    #[test]
    fn compute_layout_no_children() {
        let mut tree = LayoutTree::new();
        tree.resize(Vec2::splat(1000.0));
        let elem = Element {
            style: Style::default(),
            body: ElementBody::Text(Text::new("test", 100.0)),
        };

        let key0 = tree.push(None, elem.clone());
        let key1 = tree.push(None, elem.clone());

        tree.compute_layout();

        let layout0 = tree.layouts[&key0.0];
        let layout1 = tree.layouts[&key1.0];

        assert_eq!(layout0.position, Vec2::splat(0.0));
        assert_eq!(layout0.width, elem.bounds().min.x);
        assert_eq!(layout0.height, elem.bounds().min.y);

        assert_eq!(layout1.position, Vec2::new(0.0, layout0.height));
        assert_eq!(layout1.width, elem.bounds().min.x);
        assert_eq!(layout1.height, elem.bounds().min.y);
    }

    #[test]
    fn compute_bounds_container_growth() {
        let mut tree = LayoutTree::new();
        tree.resize(Vec2::splat(1000.0));

        let elem = Element {
            body: ElementBody::Container(),
            style: Style {
                growth: Growth(Some(1.0)),
                ..Default::default()
            },
        };

        let key = tree.push(None, elem);
        let bounds = tree.compute_bounds(key.0);

        assert_eq!(
            bounds,
            ComputedBounds {
                min: Vec2::splat(0.0),
                max: Vec2::splat(f32::INFINITY),
            }
        );
    }

    #[test]
    fn compute_bounds_container_no_growth() {
        let mut tree = LayoutTree::new();
        tree.resize(Vec2::splat(1000.0));

        let elem = Element {
            body: ElementBody::Container(),
            style: Style {
                growth: Growth(None),
                ..Default::default()
            },
        };

        let key = tree.push(None, elem);
        let bounds = tree.compute_bounds(key.0);

        assert_eq!(
            bounds,
            ComputedBounds {
                min: Vec2::splat(0.0),
                max: Vec2::splat(0.0),
            }
        );
    }

    #[test]
    fn compute_bounds_container_growth_children() {
        let mut tree = LayoutTree::new();
        tree.resize(Vec2::splat(1000.0));

        let root = Element {
            body: ElementBody::Container(),
            style: Style {
                growth: Growth(Some(1.0)),
                ..Default::default()
            },
        };
        let key = tree.push(None, root);

        let elem = Element {
            body: ElementBody::Text(Text::new("test", 100.0)),
            style: Style::default(),
        };
        tree.push(Some(key), elem.clone());

        let bounds = tree.compute_bounds(key.0);

        assert_eq!(
            bounds,
            ComputedBounds {
                min: elem.bounds().min,
                max: Vec2::splat(f32::INFINITY),
            }
        );
    }

    #[test]
    fn compute_bounds_container_no_growth_children() {
        let mut tree = LayoutTree::new();
        tree.resize(Vec2::splat(1000.0));

        let root = Element {
            body: ElementBody::Container(),
            style: Style {
                growth: Growth(None),
                ..Default::default()
            },
        };
        let key = tree.push(None, root);

        let elem = Element {
            body: ElementBody::Text(Text::new("test", 100.0)),
            style: Style::default(),
        };
        tree.push(Some(key), elem.clone());

        let bounds = tree.compute_bounds(key.0);

        assert_eq!(
            bounds,
            ComputedBounds {
                min: elem.bounds().min,
                max: elem.bounds().max,
            }
        );
    }

    #[test]
    fn compute_bounds_ignores_absolute_position() {
        let mut tree = LayoutTree::new();
        tree.resize(Vec2::splat(1000.0));

        let root = Element {
            body: ElementBody::Container(),
            style: Style {
                growth: Growth(None),
                ..Default::default()
            },
        };
        let key = tree.push(None, root);

        let elem = Element {
            body: ElementBody::Text(Text::new("test", 100.0)),
            style: Style {
                position: Position::Absolute(Vec2::splat(0.0)),
                ..Default::default()
            },
        };
        tree.push(Some(key), elem.clone());

        let bounds = tree.compute_bounds(key.0);
        assert_eq!(bounds, ComputedBounds::ZERO);
    }
}
