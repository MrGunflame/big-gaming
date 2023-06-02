use std::collections::HashMap;

use bevy_ecs::prelude::Component;
use glam::Vec2;
use slotmap::{DefaultKey, SlotMap};

use super::computed_style::{ComputedBounds, ComputedStyle};
use super::container::Container;
use super::image::Image;
use super::style::{Direction, Justify, Position, Style};
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
        style: &ComputedStyle,
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

    fn bounds(&self, style: &ComputedStyle) -> ComputedBounds {
        match &self.body {
            ElementBody::Container() => ComputedBounds::default(),
            ElementBody::Image(elem) => elem.bounds(style),
            ElementBody::Text(elem) => elem.bounds(style),
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

    pub fn len(&self) -> usize {
        self.elems.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn size(&self) -> Vec2 {
        self.size
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
            style: ComputedStyle::new(elem.style.clone(), self.size),
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
        self.changed = true;

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

    pub fn replace(&mut self, key: Key, elem: Element) {
        self.changed = true;

        self.layouts.get_mut(&key.0).unwrap().style =
            ComputedStyle::new(elem.style.clone(), self.size);
        *self.elems.get_mut(key.0).unwrap() = elem;
    }

    pub fn unchanged(&mut self) {
        self.changed = false;
    }

    pub fn is_changed(&self) -> bool {
        self.changed
    }

    pub fn compute_layout(&mut self) {
        self.computed_sizes();

        // Root behaves like an element with default styles,
        // i.e. row flow direction and start align/justify.

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
    }

    fn compute_bounds(&self, key: DefaultKey) -> ComputedBounds {
        let elem = &self.elems[key];
        let layout = &self.layouts[&key];

        let mut bounds = match &elem.body {
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

                    if elem.style.growth.x.is_some() {
                        bounds.max.x = f32::INFINITY;
                    }

                    if elem.style.growth.y.is_some() {
                        bounds.max.y = f32::INFINITY;
                    }

                    bounds
                } else {
                    // If the container can grow, it may take any size.
                    // If the can not grow, it will always have the size zero.
                    let mut bounds = ComputedBounds::ZERO;

                    if elem.style.growth.x.is_some() {
                        bounds.max.x = f32::INFINITY;
                    }

                    if elem.style.growth.y.is_some() {
                        bounds.max.y = f32::INFINITY;
                    }

                    bounds
                }
            }
            ElementBody::Image(el) => el.bounds(&layout.style),
            ElementBody::Text(el) => el.bounds(&layout.style),
        };

        let min_x = elem.style.bounds.min.x.to_pixels(self.size);
        let max_x = elem.style.bounds.max.x.to_pixels(self.size);
        let min_y = elem.style.bounds.min.y.to_pixels(self.size);
        let max_y = elem.style.bounds.max.y.to_pixels(self.size);

        bounds.min.x = f32::max(bounds.min.x, min_x);
        bounds.min.y = f32::max(bounds.min.y, min_y);

        bounds.max.x = f32::clamp(bounds.max.x, bounds.min.x, max_x);
        bounds.max.y = f32::clamp(bounds.max.y, bounds.min.y, max_y);

        debug_assert!(bounds.min.x <= bounds.max.x, "min.x <= max.x {:?}", bounds);
        debug_assert!(bounds.min.y <= bounds.max.y, "min.y <= min.y {:?}", bounds);

        let padding = layout.style.padding;
        let padding = Vec2::new(padding.left + padding.right, padding.top + padding.bottom);
        bounds.min += padding;
        bounds.max += padding;

        bounds
    }

    fn layout_element(&mut self, key: DefaultKey) {
        let elem = self.elems[key].clone();
        let layout = &self.layouts[&key];

        let mut start = layout.position;
        let mut end = Vec2::new(
            layout.position.x + layout.width,
            layout.position.y + layout.height,
        );

        // Ignore the area reserved for padding.
        start += Vec2::new(layout.style.padding.top, layout.style.padding.left);
        end -= Vec2::new(layout.style.padding.bottom, layout.style.padding.right);

        let width = layout.width - layout.style.padding.left - layout.style.padding.right;
        let height = layout.height - layout.style.padding.top - layout.style.padding.bottom;

        if let Some(children) = self.children.get(&key).cloned() {
            match elem.style.justify {
                Justify::Start => {
                    let mut next_position = start;
                    let size_per_elem =
                        size_per_element(end - start, children.len() as u32, elem.style.direction);

                    for child in children {
                        let child_style = &self.elems[child].style;

                        let bounds = self.compute_bounds(child);
                        let layout = self.layouts.get_mut(&child).unwrap();

                        match child_style.position {
                            Position::Relative => {
                                layout.position = next_position;
                                layout.width =
                                    f32::clamp(size_per_elem.x, bounds.min.x, bounds.max.x);
                                layout.height =
                                    f32::clamp(size_per_elem.y, bounds.min.y, bounds.max.y);

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
                Justify::End => {
                    let mut next_position = start
                        + match elem.style.direction {
                            Direction::Row => Vec2::new(0.0, height),
                            Direction::Column => Vec2::new(width, 0.0),
                        };

                    let size_per_elem =
                        size_per_element(end - start, children.len() as u32, elem.style.direction);

                    for child in children.iter().rev().copied() {
                        let child_style = &self.elems[child].style;

                        let bounds = self.compute_bounds(child);
                        let layout = self.layouts.get_mut(&child).unwrap();

                        match child_style.position {
                            Position::Relative => {
                                layout.position = next_position;
                                layout.width =
                                    f32::clamp(size_per_elem.x, bounds.min.x, bounds.max.x);
                                layout.height =
                                    f32::clamp(size_per_elem.y, bounds.min.y, bounds.max.y);

                                match elem.style.direction {
                                    Direction::Row => next_position.y -= layout.height,
                                    Direction::Column => next_position.x -= layout.width,
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

                        // Note that we don't know the size of the element we're rendering
                        // before we actually render it.
                        // Because of that we start rendering the end of the allocated space
                        // and then move it back the by size that it rendered.
                        let layout = self.layouts.get_mut(&child).unwrap();
                        match elem.style.direction {
                            Direction::Row => {
                                layout.position.y -= layout.height;
                            }
                            Direction::Column => {
                                layout.position.x -= layout.width;
                            }
                        }

                        self.layout_element(child);
                    }
                }
                Justify::Center => {
                    let mut next_position = start;
                    let size_per_elem =
                        size_per_element(end - start, children.len() as u32, elem.style.direction);

                    for child in children {
                        let child_style = &self.elems[child].style;

                        let bounds = self.compute_bounds(child);
                        let layout = self.layouts.get_mut(&child).unwrap();

                        match child_style.position {
                            Position::Relative => {
                                layout.position = next_position;
                                layout.width =
                                    f32::clamp(size_per_elem.x, bounds.min.x, bounds.max.x);
                                layout.height =
                                    f32::clamp(size_per_elem.y, bounds.min.y, bounds.max.y);

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

                    let elem = self.elems[key].clone();
                    let children = self.children.get(&key).unwrap();

                    // The first element is spaced at `layout.position` and the
                    // end of the last element at `next_position`.
                    let allocated_space = next_position - start;

                    match elem.style.direction {
                        Direction::Row => {
                            let total_size = (start.y + height) - start.y;

                            let mut next_pos = start.y + ((total_size - allocated_space.y) / 2.0);

                            for child in children {
                                let layout = self.layouts.get_mut(child).unwrap();
                                layout.position.y = next_pos;

                                next_pos += layout.height;
                            }
                        }
                        Direction::Column => {
                            let total_size = (start.x + width) - start.x;
                            let mut next_pos = start.x + ((total_size - allocated_space.x) / 2.0);

                            for child in children {
                                let layout = self.layouts.get_mut(child).unwrap();
                                layout.position.x = next_pos;

                                next_pos += layout.width;
                            }
                        }
                    }

                    for child in children.clone() {
                        self.layout_element(child);
                    }
                }
                Justify::SpaceBetween => {
                    let mut next_position = start;
                    let size_per_elem =
                        size_per_element(end - start, children.len() as u32, elem.style.direction);

                    for child in children {
                        let child_style = &self.elems[child].style;

                        let bounds = self.compute_bounds(child);
                        let layout = self.layouts.get_mut(&child).unwrap();

                        match child_style.position {
                            Position::Relative => {
                                layout.position = next_position;
                                layout.width =
                                    f32::clamp(size_per_elem.x, bounds.min.x, bounds.max.x);
                                layout.height =
                                    f32::clamp(size_per_elem.y, bounds.min.y, bounds.max.y);

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

                    let elem = self.elems[key].clone();
                    let children = self.children.get(&key).unwrap();

                    // The first element is spaced at `layout.position` and the
                    // end of the last element at `next_position`.
                    let allocated_space = next_position - start;

                    match elem.style.direction {
                        Direction::Row => {
                            let total_size = (start.y + height) - start.y;

                            // Distance/emtpy space between elements.
                            let distance = (total_size - allocated_space.y)
                                / children.len().saturating_sub(1) as f32;

                            let mut next_pos = start.y;

                            for child in children {
                                let layout = self.layouts.get_mut(child).unwrap();
                                layout.position.y = next_pos;

                                next_pos += layout.height;
                                next_pos += distance;
                            }
                        }
                        Direction::Column => {
                            let total_size = (start.x + height) - start.x;

                            let distance = (total_size - allocated_space.x)
                                / children.len().saturating_sub(1) as f32;

                            let mut next_pos = start.x;

                            for child in children {
                                let layout = self.layouts.get_mut(child).unwrap();
                                layout.position.x = next_pos;

                                next_pos += layout.width;
                                next_pos += distance;
                            }
                        }
                    }

                    for child in children.clone() {
                        self.layout_element(child);
                    }
                }
                Justify::SpaceAround => {
                    let mut next_position = start;
                    let size_per_elem =
                        size_per_element(end - start, children.len() as u32, elem.style.direction);

                    for child in children {
                        let child_style = &self.elems[child].style;

                        let bounds = self.compute_bounds(child);
                        let layout = self.layouts.get_mut(&child).unwrap();

                        match child_style.position {
                            Position::Relative => {
                                layout.position = next_position;
                                layout.width =
                                    f32::clamp(size_per_elem.x, bounds.min.x, bounds.max.x);
                                layout.height =
                                    f32::clamp(size_per_elem.y, bounds.min.y, bounds.max.y);

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

                    let elem = self.elems[key].clone();
                    let children = self.children.get(&key).unwrap();

                    // The first element is spaced at `layout.position` and the
                    // end of the last element at `next_position`.
                    let allocated_space = next_position - start;

                    match elem.style.direction {
                        Direction::Row => {
                            let total_size = (start.y + height) - start.y;

                            // Distance/emtpy space betweem elements and borders.
                            // Note that overflows not possible as that would require
                            // more that `isize::MAX` children to be allocated, which
                            // is not allowed.
                            let distance =
                                (total_size - allocated_space.y) / (children.len() + 1) as f32;

                            let mut next_pos = start.y + distance;

                            for child in children {
                                let layout = self.layouts.get_mut(child).unwrap();
                                layout.position.y = next_pos;

                                next_pos += layout.height;
                                next_pos += distance;
                            }
                        }
                        Direction::Column => {
                            let total_size = (start.x + width) - start.x;

                            let distance =
                                (total_size - allocated_space.x) / (children.len() + 1) as f32;

                            let mut next_pos = start.x + distance;

                            for child in children {
                                let layout = self.layouts.get_mut(child).unwrap();
                                layout.position.x = next_pos;

                                next_pos += layout.width;
                                next_pos += distance;
                            }
                        }
                    }

                    for child in children.clone() {
                        self.layout_element(child);
                    }
                }
            }
        }
    }

    fn computed_sizes(&mut self) {
        for (key, elem) in self.elems.iter() {
            let layout = self.layouts.get_mut(&key).unwrap();

            layout.style = ComputedStyle::new(elem.style.clone(), self.size);

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

#[derive(Clone, Debug)]
pub struct Layout {
    pub style: ComputedStyle,

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

    use crate::render::computed_style::ComputedStyle;
    use crate::render::layout::ComputedBounds;
    use crate::render::style::{
        Bounds, Direction, Growth, Justify, Padding, Position, Size, SizeVec2, Style,
    };
    use crate::render::{BuildPrimitiveElement, Text};

    use super::{size_per_element, Element, ElementBody, Key, LayoutTree};

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

        let style = Style::default();

        let elem = Element {
            style: style.clone(),
            body: ElementBody::Text(Text::new("test", 100.0)),
        };

        let key0 = tree.push(None, elem.clone());
        let key1 = tree.push(None, elem.clone());

        tree.compute_layout();

        let layout0 = &tree.layouts[&key0.0];
        let layout1 = &tree.layouts[&key1.0];

        let style = ComputedStyle::new(style, Vec2::splat(1000.0));

        assert_eq!(layout0.position, Vec2::splat(0.0));
        assert_eq!(layout0.width, elem.bounds(&style).min.x);
        assert_eq!(layout0.height, elem.bounds(&style).min.y);

        assert_eq!(layout1.position, Vec2::new(0.0, layout0.height));
        assert_eq!(layout1.width, elem.bounds(&style).min.x);
        assert_eq!(layout1.height, elem.bounds(&style).min.y);
    }

    #[test]
    fn compute_bounds_container_growth() {
        let mut tree = LayoutTree::new();
        tree.resize(Vec2::splat(1000.0));

        let elem = Element {
            body: ElementBody::Container(),
            style: Style {
                growth: Growth::splat(1.0),
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
                growth: Growth::NONE,
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

        let style = Style {
            growth: Growth::splat(1.0),
            ..Default::default()
        };

        let root = Element {
            body: ElementBody::Container(),
            style: style.clone(),
        };
        let key = tree.push(None, root);

        let elem = Element {
            body: ElementBody::Text(Text::new("test", 100.0)),
            style: Style::default(),
        };
        tree.push(Some(key), elem.clone());

        let bounds = tree.compute_bounds(key.0);

        let style = ComputedStyle::new(style, Vec2::splat(1000.0));

        assert_eq!(
            bounds,
            ComputedBounds {
                min: elem.bounds(&style).min,
                max: Vec2::splat(f32::INFINITY),
            }
        );
    }

    #[test]
    fn compute_bounds_container_no_growth_children() {
        let mut tree = LayoutTree::new();
        tree.resize(Vec2::splat(1000.0));

        let style = Style {
            growth: Growth::NONE,
            ..Default::default()
        };

        let root = Element {
            body: ElementBody::Container(),
            style: style.clone(),
        };
        let key = tree.push(None, root);

        let elem = Element {
            body: ElementBody::Text(Text::new("test", 100.0)),
            style: Style::default(),
        };
        tree.push(Some(key), elem.clone());

        let bounds = tree.compute_bounds(key.0);

        let style = ComputedStyle::new(style, Vec2::splat(1000.0));

        assert_eq!(
            bounds,
            ComputedBounds {
                min: elem.bounds(&style).min,
                max: elem.bounds(&style).max,
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
                growth: Growth::NONE,
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

    #[test]
    fn compute_bounds_exact_size() {
        let mut tree = LayoutTree::new();
        tree.resize(Vec2::splat(1000.0));

        let bounds = Bounds {
            min: SizeVec2::splat(Size::Pixels(10.0)),
            max: SizeVec2::splat(Size::Pixels(10.0)),
        };

        let key = tree.push(
            None,
            Element {
                body: ElementBody::Container(),
                style: Style {
                    bounds,
                    ..Default::default()
                },
            },
        );

        let bounds = tree.compute_bounds(key.0);

        assert_eq!(bounds.min, Vec2::splat(10.0));
        assert_eq!(bounds.max, Vec2::splat(10.0));
    }

    #[test]
    fn compute_layout_exact_size_root() {
        let mut tree = LayoutTree::new();
        tree.resize(Vec2::splat(1000.0));

        let bounds = Bounds {
            min: SizeVec2::splat(Size::Pixels(10.0)),
            max: SizeVec2::splat(Size::Pixels(10.0)),
        };

        let key = tree.push(
            None,
            Element {
                body: ElementBody::Container(),
                style: Style {
                    bounds,
                    ..Default::default()
                },
            },
        );

        tree.compute_layout();

        let layout = &tree.layouts[&key.0];
        assert_eq!(layout.height, 10.0);
        assert_eq!(layout.width, 10.0);
    }

    #[test]
    fn compute_layout_exact_size_children() {
        let mut tree = LayoutTree::new();
        tree.resize(Vec2::splat(1000.0));

        let bounds = Bounds {
            min: SizeVec2::splat(Size::Pixels(10.0)),
            max: SizeVec2::splat(Size::Pixels(10.0)),
        };

        let root = tree.push(
            None,
            Element {
                body: ElementBody::Container(),
                style: Style::default(),
            },
        );

        let key = tree.push(
            Some(root),
            Element {
                body: ElementBody::Container(),
                style: Style {
                    bounds,
                    ..Default::default()
                },
            },
        );

        tree.compute_layout();

        let layout = &tree.layouts[&key.0];
        assert_eq!(layout.height, 10.0);
        assert_eq!(layout.width, 10.0);
    }

    fn create_justify_test(
        direction: Direction,
        justify: Justify,
        num_elems: u32,
        size: f32,
    ) -> (LayoutTree, Vec<Key>) {
        let mut tree = LayoutTree::new();
        tree.resize(Vec2::splat(1000.0));

        let root = tree.push(
            None,
            Element {
                body: ElementBody::Container(),
                style: Style {
                    // Claim entire viewport so we can actually test
                    // child positions.
                    bounds: Bounds {
                        min: SizeVec2::splat(Size::Pixels(1000.0)),
                        max: SizeVec2::splat(Size::Pixels(1000.0)),
                    },
                    direction,
                    justify,
                    ..Default::default()
                },
            },
        );

        let elem = Element {
            body: ElementBody::Container(),
            style: Style {
                bounds: Bounds {
                    min: SizeVec2::splat(Size::Pixels(size)),
                    max: SizeVec2::splat(Size::Pixels(size)),
                },
                ..Default::default()
            },
        };

        let keys = (0..num_elems)
            .map(|_| tree.push(Some(root), elem.clone()))
            .collect();

        tree.compute_layout();

        (tree, keys)
    }

    #[test]
    fn compute_layout_row_justify_start() {
        let size = 10.0;

        let (tree, keys) = create_justify_test(Direction::Row, Justify::Start, 3, size);

        let mut offset = 0.0;
        for key in keys {
            let layout = &tree.layouts[&key.0];

            assert_eq!(layout.position, Vec2::new(0.0, offset));

            offset += size;
        }
    }

    #[test]
    fn compute_layout_column_justify_start() {
        let size = 10.0;

        let (tree, keys) = create_justify_test(Direction::Column, Justify::Start, 3, size);

        let mut offset = 0.0;
        for key in keys {
            let layout = &tree.layouts[&key.0];

            assert_eq!(layout.position, Vec2::new(offset, 0.0));

            offset += size;
        }
    }

    #[test]
    fn compute_layout_row_justify_end() {
        let size = 10.0;

        let (tree, keys) = create_justify_test(Direction::Row, Justify::End, 3, size);

        let mut offset = 1000.0 - (size * 3.0);
        for key in keys {
            let layout = &tree.layouts[&key.0];

            assert_eq!(layout.position, Vec2::new(0.0, offset));

            offset += size;
        }
    }

    #[test]
    fn compute_layout_column_justify_end() {
        let size = 10.0;

        let (tree, keys) = create_justify_test(Direction::Column, Justify::End, 3, size);

        let mut offset = 1000.0 - (size * 3.0);
        for key in keys {
            let layout = &tree.layouts[&key.0];

            assert_eq!(layout.position, Vec2::new(offset, 0.0));

            offset += size;
        }
    }

    #[test]
    fn compute_layout_row_justify_center() {
        let size = 10.0;

        let (tree, keys) = create_justify_test(Direction::Row, Justify::Center, 3, size);

        let mut offset = (1000.0 - (10.0 * 3.0)) / 2.0;
        for key in keys {
            let layout = &tree.layouts[&key.0];

            assert_eq!(layout.position, Vec2::new(0.0, offset));

            offset += size;
        }
    }

    #[test]
    fn compute_layout_column_justify_center() {
        let size = 10.0;

        let (tree, keys) = create_justify_test(Direction::Column, Justify::Center, 3, size);

        let mut offset = (1000.0 - (10.0 * 3.0)) / 2.0;
        for key in keys {
            let layout = &tree.layouts[&key.0];

            assert_eq!(layout.position, Vec2::new(offset, 0.0));

            offset += size;
        }
    }

    #[test]
    fn compute_layout_row_justify_space_between() {
        let size = 10.0;

        let (tree, keys) = create_justify_test(Direction::Row, Justify::SpaceBetween, 3, size);

        let distance = (1000.0 - (size * 3.0)) / 2.0;

        let mut offset = 0.0;
        for key in keys {
            let layout = &tree.layouts[&key.0];

            assert_eq!(layout.position, Vec2::new(0.0, offset));

            offset += size + distance;
        }
    }

    #[test]
    fn compute_layout_column_justify_space_between() {
        let size = 10.0;

        let (tree, keys) = create_justify_test(Direction::Column, Justify::SpaceBetween, 3, size);

        let distance = (1000.0 - (size * 3.0)) / 2.0;

        let mut offset = 0.0;
        for key in keys {
            let layout = &tree.layouts[&key.0];

            assert_eq!(layout.position, Vec2::new(offset, 0.0));

            offset += size + distance;
        }
    }

    #[test]
    fn compute_layout_row_justify_space_around() {
        let size = 10.0;

        let (tree, keys) = create_justify_test(Direction::Row, Justify::SpaceAround, 3, size);

        let distance = (1000.0 - (size * 3.0)) / 4.0;

        let mut offset = distance;
        for key in keys {
            let layout = &tree.layouts[&key.0];

            assert_eq!(layout.position, Vec2::new(0.0, offset));

            offset += size + distance;
        }
    }

    #[test]
    fn compute_layout_column_justify_space_around() {
        let size = 10.0;

        let (tree, keys) = create_justify_test(Direction::Column, Justify::SpaceAround, 3, size);

        let distance = (1000.0 - (size * 3.0)) / 4.0;

        let mut offset = distance;
        for key in keys {
            let layout = &tree.layouts[&key.0];

            assert_eq!(layout.position, Vec2::new(offset, 0.0));

            offset += size + distance;
        }
    }

    #[test]
    fn compute_layout_padding_no_children() {
        let mut tree = LayoutTree::new();
        tree.resize(Vec2::splat(1000.0));

        let key = tree.push(
            None,
            Element {
                body: ElementBody::Container(),
                style: Style {
                    bounds: Bounds {
                        min: SizeVec2::splat(Size::Pixels(10.0)),
                        max: SizeVec2::splat(Size::Pixels(10.0)),
                    },
                    padding: Padding::splat(Size::Pixels(10.0)),
                    ..Default::default()
                },
            },
        );

        tree.compute_layout();

        let layout = &tree.layouts[&key.0];

        assert_eq!(layout.position, Vec2::splat(0.0));
    }

    #[test]
    fn computed_layout_padding_with_children() {
        let mut tree = LayoutTree::new();
        tree.resize(Vec2::splat(1000.0));

        let root = tree.push(
            None,
            Element {
                body: ElementBody::Container(),
                style: Style {
                    padding: Padding::splat(Size::Pixels(10.0)),
                    ..Default::default()
                },
            },
        );

        let elem = Element {
            body: ElementBody::Container(),
            style: Style {
                bounds: Bounds {
                    min: SizeVec2::splat(Size::Pixels(10.0)),
                    max: SizeVec2::splat(Size::Pixels(10.0)),
                },
                ..Default::default()
            },
        };

        let keys: Vec<_> = (0..3)
            .map(|_| tree.push(Some(root), elem.clone()))
            .collect();

        tree.compute_layout();

        let mut offset = Vec2::new(10.0, 10.0);
        for key in keys {
            let layout = &tree.layouts[&key.0];

            assert_eq!(layout.position, offset);

            offset.y += 10.0;
        }
    }
}
