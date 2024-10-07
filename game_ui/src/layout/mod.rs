pub mod computed_style;
pub mod debug_dump;

use std::collections::{BTreeMap, HashMap};

use glam::UVec2;

use self::computed_style::{ComputedBounds, ComputedStyle};
use crate::primitive::Primitive;
use crate::render::Rect;
use crate::style::{Direction, Justify, Position};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(u64);

#[derive(Clone, Debug)]
pub struct LayoutTree {
    next_id: u64,
    // Note that the order is important; we want to render elements
    // inserted after their parents to be render after the parent
    // was rendered.
    elems: BTreeMap<Key, Primitive>,
    layouts: HashMap<Key, Layout>,
    size: UVec2,
    scale_factor: f64,
    changed: bool,

    // parent => vec![child]
    children: HashMap<Key, Vec<Key>>,
    // child => parent
    parents: HashMap<Key, Key>,
    /// Root elements
    root: Vec<Key>,
}

impl LayoutTree {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            elems: BTreeMap::new(),
            layouts: HashMap::new(),
            size: UVec2::splat(0),
            scale_factor: 1.0,
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

    pub fn size(&self) -> UVec2 {
        self.size
    }

    pub fn resize(&mut self, size: UVec2) {
        self.size = size;
        self.changed = true;

        for layout in self.layouts.values_mut() {
            layout.has_changed = true;
        }
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
        self.changed = true;

        for layout in self.layouts.values_mut() {
            layout.has_changed = true;
        }
    }

    pub fn children(&self, key: Key) -> Option<&[Key]> {
        self.children.get(&key).map(|v| v.as_slice())
    }

    /// Inserts a new [`Element`] at the given `index`.
    ///
    /// # Panics
    ///
    /// Panics if `index < parent_len`.
    pub fn insert(&mut self, parent: Option<Key>, elem: Primitive, index: usize) -> Key {
        let layout = Layout {
            position: UVec2::ZERO,
            height: 0,
            width: 0,
            style: ComputedStyle::new(elem.style.clone(), self.size, self.scale_factor),
            has_changed: true,
        };

        let key = Key(self.next_id);
        self.next_id += 1;
        self.elems.insert(key, elem);
        self.layouts.insert(key, layout);

        self.children.insert(key, vec![]);

        if let Some(parent) = parent {
            let children = self.children.get_mut(&parent).unwrap();
            children.insert(index, parent);
        } else {
            self.root.insert(index, key);
        }

        self.changed = true;
        key
    }

    pub fn push(&mut self, parent: Option<Key>, elem: Primitive) -> Key {
        let layout = Layout {
            position: UVec2::ZERO,
            height: 0,
            width: 0,
            style: ComputedStyle::new(elem.style.clone(), self.size, self.scale_factor),
            has_changed: true,
        };

        let key = Key(self.next_id);
        self.next_id += 1;
        self.elems.insert(key, elem);
        self.layouts.insert(key, layout);

        self.children.insert(key, vec![]);

        if let Some(parent) = parent {
            self.children.get_mut(&parent).unwrap().push(key);
            self.parents.insert(key, parent);
        } else {
            self.root.push(key);
        }

        self.changed = true;

        key
    }

    pub fn remove(&mut self, key: Key) {
        self.changed = true;

        self.elems.remove(&key);
        self.layouts.remove(&key);

        self.root.retain(|k| *k != key);

        if let Some(children) = self.children.remove(&key) {
            for c in children {
                self.remove(c);
            }
        }

        if let Some(parent) = self.parents.remove(&key) {
            if let Some(children) = self.children.get_mut(&parent) {
                children.retain(|k| *k != key);
            }
        }
    }

    pub fn replace(&mut self, key: Key, elem: Primitive) {
        if !self.elems.contains_key(&key) {
            return;
        }

        self.changed = true;

        let layout = self.layouts.get_mut(&key).unwrap();
        layout.style = ComputedStyle::new(elem.style.clone(), self.size, self.scale_factor);
        // Set the `has_changed` flag.
        // This will cause the element to be completely rerendered
        // even if no layout properties are changed.
        // If this flag was not set, it would be possible that the new
        // style would never applied when the layout doesn't change.
        layout.has_changed = true;

        *self.elems.get_mut(&key).unwrap() = elem;
    }

    pub fn is_changed(&self) -> bool {
        self.changed
    }

    pub fn compute_layout(&mut self) {
        if !self.changed {
            return;
        }
        self.changed = false;

        self.computed_sizes();

        // Root behaves like an element with default styles,
        // i.e. row flow direction and start align/justify.

        let mut next_position = UVec2::ZERO;
        let size_per_elem = size_per_element(self.size, self.root.len() as u32, Direction::Row);
        // FIXME: No need to clone, layout_element doesn't touch self.root.
        for key in &self.root.clone() {
            let bounds = self.compute_bounds(*key);

            let layout = self.layouts.get_mut(key).unwrap();

            // Every elements gets `size_per_elem` or `max`, whichever is lower.
            let position = next_position;
            let width = u32::clamp(size_per_elem.x, bounds.min.x, bounds.max.x);
            let height = u32::clamp(size_per_elem.y, bounds.min.y, bounds.max.y);

            // Only if the layout has changed from the previous layout
            // will we set the `has_changed` flag.
            // This is useful as a hint to allow the renderer skip recreating
            // elements that have not changed.
            if layout.position != position || layout.width != width || layout.height != height {
                layout.has_changed = true;
            }

            layout.position = position;
            layout.width = width;
            layout.height = height;

            next_position.y += layout.height;

            self.layout_element(*key);
        }
    }

    /// Computes the bounds for the element.
    fn compute_bounds(&self, key: Key) -> ComputedBounds {
        // The caller should guarantee that the key exists.
        let elem = self.elems.get(&key).unwrap();
        let layout = self.layouts.get(&key).unwrap();

        let mut bounds = if let Some(children) = self.children.get(&key) {
            let mut bounds = elem.bounds(&layout.style, self.scale_factor);
            for key in children {
                // Elements with absolute position are excluded.
                let child = self.elems.get(key).unwrap();
                if child.style.position.is_absolute() {
                    continue;
                }

                let child_bounds = self.compute_bounds(*key);

                let min = child_bounds.min;
                let max = child_bounds.max;

                match elem.style.direction {
                    Direction::Row => {
                        bounds.min.y = bounds.min.y.saturating_add(min.y);
                        bounds.min.x = u32::max(bounds.min.x, min.x);

                        bounds.max.y = bounds.max.y.saturating_add(max.y);
                        bounds.max.x = u32::max(bounds.max.x, max.x);
                    }
                    Direction::Column => {
                        bounds.min.y = u32::max(bounds.min.y, min.y);
                        bounds.min.x = bounds.min.x.saturating_add(min.x);

                        bounds.max.y = u32::max(bounds.max.y, max.y);
                        bounds.max.x = bounds.max.x.saturating_add(max.x);
                    }
                }
            }

            if elem.style.growth.x.is_some() {
                bounds.max.x = u32::MAX;
            }

            if elem.style.growth.y.is_some() {
                bounds.max.y = u32::MAX;
            }

            bounds
        } else {
            // If the container can grow, it may take any size.
            // If the can not grow, it will always have the size zero.
            let mut bounds = elem.bounds(&layout.style, self.scale_factor);

            if elem.style.growth.x.is_some() {
                bounds.max.x = u32::MAX;
            }

            if elem.style.growth.y.is_some() {
                bounds.max.y = u32::MAX;
            }

            bounds
        };

        // Clamp the actual bounds between the wanted bounds.
        let min_x = elem
            .style
            .bounds
            .min
            .x
            .to_pixels(self.size, self.scale_factor);
        let min_y = elem
            .style
            .bounds
            .min
            .y
            .to_pixels(self.size, self.scale_factor);
        let max_x = elem
            .style
            .bounds
            .max
            .x
            .to_pixels(self.size, self.scale_factor);
        let max_y = elem
            .style
            .bounds
            .max
            .y
            .to_pixels(self.size, self.scale_factor);

        // The min bounds are the specified min bounds of the element, or the
        // min bounds of the children.
        bounds.min.x = u32::max(bounds.min.x, min_x);
        bounds.min.y = u32::max(bounds.min.y, min_y);

        // If min bounds > max bounds we clamp the min bounds to
        // max.
        // FIXME: Is this actually what we want? Define what happens
        // if min > max.
        bounds.min.x = u32::min(bounds.min.x, max_x);
        bounds.min.y = u32::min(bounds.min.y, max_y);

        bounds.max.x = u32::clamp(bounds.max.x, bounds.min.x, max_x);
        bounds.max.y = u32::clamp(bounds.max.y, bounds.min.y, max_y);

        debug_assert!(bounds.min.x <= bounds.max.x);
        debug_assert!(bounds.min.y <= bounds.max.y);

        // Apply the wanted padding on top of the min/max bounds.
        let padding = layout.style.padding;
        let padding = UVec2::new(padding.left + padding.right, padding.top + padding.bottom);
        bounds.min = bounds.min.saturating_add(padding);
        bounds.max = bounds.max.saturating_add(padding);

        bounds
    }

    fn layout_element(&mut self, key: Key) {
        let elem = self.elems.get(&key).unwrap().clone();
        let layout = self.layouts.get(&key).unwrap();

        // Start and end points of the content.
        // FIXME: What happens if padding > size?
        let content = Rect {
            min: UVec2 {
                x: layout.position.x + layout.style.padding.left,
                y: layout.position.y + layout.style.padding.top,
            },
            max: UVec2 {
                x: layout.position.x + layout.width - layout.style.padding.right,
                y: layout.position.y + layout.height - layout.style.padding.bottom,
            },
        };

        if let Some(children) = self.children.get(&key).cloned() {
            // Relative positioned children
            let relative_children: u32 = children
                .iter()
                .map(|key| self.elems.get(key).unwrap().style.position)
                .filter(|p| p.is_relative())
                .count() as u32;

            let size_per_elem =
                size_per_element(content.size(), relative_children, elem.style.direction);

            let mut child_bounds = vec![];

            for child in &children {
                let child_style = &self.elems.get(child).unwrap().style;

                // Absolute elements are excluded in the parent layout computation.
                if child_style.position.is_relative() {
                    let bounds = self.compute_bounds(*child);

                    let width = u32::clamp(size_per_elem.x, bounds.min.x, bounds.max.x);
                    let height = u32::clamp(size_per_elem.y, bounds.min.y, bounds.max.y);

                    child_bounds.push((child, UVec2::new(width, height)));
                }
            }

            match elem.style.justify {
                Justify::Start => {
                    let mut next_position = content.min;

                    for (key, bounds) in child_bounds {
                        let layout = self.layouts.get_mut(key).unwrap();

                        layout.position = next_position;
                        layout.width = bounds.x;
                        layout.height = bounds.y;

                        match elem.style.direction {
                            Direction::Row => next_position.y += layout.height,
                            Direction::Column => next_position.x += layout.width,
                        }
                    }
                }
                Justify::End => {
                    let mut offset = 0;
                    for (_, bounds) in &child_bounds {
                        match elem.style.direction {
                            Direction::Row => offset += bounds.x,
                            Direction::Column => offset += bounds.y,
                        }
                    }

                    let mut next_position = content.min
                        + match elem.style.direction {
                            Direction::Row => UVec2::new(0, content.height() - offset),
                            Direction::Column => UVec2::new(content.width() - offset, 0),
                        };

                    for (key, bounds) in child_bounds {
                        let layout = self.layouts.get_mut(key).unwrap();

                        layout.position = next_position;
                        layout.width = bounds.x;
                        layout.height = bounds.y;

                        match elem.style.direction {
                            Direction::Row => next_position.y += layout.height,
                            Direction::Column => next_position.x += layout.width,
                        }
                    }
                }
                Justify::Center => {
                    let mut offset = 0;
                    for (_, bounds) in &child_bounds {
                        match elem.style.direction {
                            Direction::Row => offset += bounds.x,
                            Direction::Column => offset += bounds.y,
                        }
                    }

                    let mut next_position = content.min
                        + match elem.style.direction {
                            Direction::Row => {
                                UVec2::new(0, content.height().saturating_sub(offset) / 2)
                            }
                            Direction::Column => {
                                UVec2::new(content.width().saturating_sub(offset) / 2, 0)
                            }
                        };

                    for (key, bounds) in child_bounds {
                        let layout = self.layouts.get_mut(key).unwrap();

                        layout.position = next_position;
                        layout.width = bounds.x;
                        layout.height = bounds.y;

                        match elem.style.direction {
                            Direction::Row => next_position.y += layout.height,
                            Direction::Column => next_position.x += layout.width,
                        }
                    }
                }
                Justify::SpaceBetween => {
                    let num_children = child_bounds.len() as u32;

                    if num_children > 0 {
                        let mut offset = 0;
                        for (_, bounds) in &child_bounds {
                            match elem.style.direction {
                                Direction::Row => offset += bounds.x,
                                Direction::Column => offset += bounds.y,
                            }
                        }

                        let pad_zone = match elem.style.direction {
                            Direction::Row => content
                                .height()
                                .saturating_sub(offset)
                                .checked_div(num_children - 1)
                                .unwrap_or(content.height().saturating_sub(offset)),
                            Direction::Column => content
                                .width()
                                .saturating_sub(offset)
                                .checked_div(num_children - 1)
                                .unwrap_or(content.height().saturating_sub(offset)),
                        };

                        let mut next_position = content.min;

                        for (key, bounds) in child_bounds {
                            let layout = self.layouts.get_mut(key).unwrap();

                            layout.position = next_position;
                            layout.width = bounds.x;
                            layout.height = bounds.y;

                            match elem.style.direction {
                                Direction::Row => {
                                    next_position.y += layout.height;
                                    next_position.y += pad_zone;
                                }
                                Direction::Column => {
                                    next_position.x += layout.width;
                                    next_position.x += pad_zone;
                                }
                            }
                        }
                    }
                }
                Justify::SpaceAround => {
                    let num_children = child_bounds.len() as u32;

                    if num_children > 0 {
                        let mut offset = 0;
                        for (_, bounds) in &child_bounds {
                            match elem.style.direction {
                                Direction::Row => offset += bounds.x,
                                Direction::Column => offset += bounds.y,
                            }
                        }

                        let pad_zone = match elem.style.direction {
                            Direction::Row => {
                                content.height().saturating_sub(offset) / (num_children + 1)
                            }
                            Direction::Column => {
                                content.width().saturating_sub(offset) / (num_children + 1)
                            }
                        };

                        let mut next_position = content.min;
                        match elem.style.direction {
                            Direction::Row => next_position.y += pad_zone,
                            Direction::Column => next_position.x += pad_zone,
                        };

                        for (key, bounds) in child_bounds {
                            let layout = self.layouts.get_mut(key).unwrap();

                            layout.position = next_position;
                            layout.width = bounds.x;
                            layout.height = bounds.y;

                            match elem.style.direction {
                                Direction::Row => {
                                    next_position.y += layout.height;
                                    next_position.y += pad_zone;
                                }
                                Direction::Column => {
                                    next_position.x += layout.width;
                                    next_position.x += pad_zone;
                                }
                            }
                        }
                    }
                }
            }

            let mut abs_child_bounds = vec![];
            for child in &children {
                let child_style = &self.elems.get(child).unwrap().style;

                if let Position::Absolute(pos) = child_style.position {
                    let bounds = self.compute_bounds(*child);

                    let width = u32::clamp(self.size.x, bounds.min.x, bounds.max.x);
                    let height = u32::clamp(self.size.y, bounds.min.y, bounds.max.y);

                    abs_child_bounds.push((child, pos, UVec2::new(width, height)));
                }
            }

            for (key, pos, bounds) in abs_child_bounds {
                let layout = self.layouts.get_mut(key).unwrap();

                layout.position = pos;
                layout.width = bounds.x;
                layout.height = bounds.y;
            }

            for child in children {
                if cfg!(debug_assertions) {
                    let layout = &self.layouts[&child];
                    let bounds = layout.style.bounds;

                    assert!(layout.width >= bounds.min.x && layout.width <= bounds.max.x);
                    assert!(layout.height >= bounds.min.y && layout.height <= bounds.max.y);
                }

                self.layout_element(child);
            }
        }
    }

    fn computed_sizes(&mut self) {
        for (key, elem) in self.elems.iter() {
            let layout = self.layouts.get_mut(key).unwrap();

            let mut style = ComputedStyle::new(elem.style.clone(), self.size, self.scale_factor);
            style.bounds = ComputedBounds::new(elem.style.bounds, self.size, self.scale_factor);

            // Only set the `has_changed` flag if the computed style has changed
            // from the previous computed style.
            // Note that comparing the underlying `style` of the `ComputedStyle`
            // is not necessary since it never changes unless it was modified
            // by an accessor method, in which case the `has_changed` flag is
            // already set to `true`.
            if !layout.style.equal_except_style(&style) {
                layout.has_changed = true;
            }

            layout.style = style;
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

    pub fn keys(&self) -> impl Iterator<Item = Key> + '_ {
        self.elems.keys().copied()
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut Primitive> {
        match self.elems.get_mut(&key) {
            Some(elem) => {
                self.changed = true;
                Some(elem)
            }
            None => None,
        }
    }

    pub fn layout(&self, key: Key) -> Option<&Layout> {
        self.layouts.get(&key)
    }

    /// Marks the [`Layout`] of all elements of not being changed.
    fn mark_all_as_unchanged(&mut self) {
        for layout in self.layouts.values_mut() {
            layout.has_changed = false;
        }
    }

    pub fn collect_all(&mut self) -> Vec<(Key, Layout, Primitive)> {
        let vec = self
            .keys()
            .zip(self.layouts())
            .zip(self.elements())
            .map(|((key, layout), elements)| (key, layout.clone(), elements.clone()))
            .collect();
        self.mark_all_as_unchanged();
        vec
    }
}

impl Default for LayoutTree {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct Elements<'a> {
    iter: std::collections::btree_map::Iter<'a, Key, Primitive>,
}

impl<'a> Iterator for Elements<'a> {
    type Item = &'a Primitive;

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
    keys: std::collections::btree_map::Iter<'a, Key, Primitive>,
    layouts: &'a HashMap<Key, Layout>,
}

impl<'a> Iterator for Layouts<'a> {
    type Item = &'a Layout;

    fn next(&mut self) -> Option<Self::Item> {
        self.keys.next().map(|(k, _)| self.layouts.get(k).unwrap())
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

    pub position: UVec2,
    pub width: u32,
    pub height: u32,

    /// `true` if any property of the [`Primitive`] that this `Layout` is associated with has
    /// changed.
    ///
    /// If `false` the renderer may decide to reuse the same render command as in the last frame.
    ///
    /// Note also that `has_changed` is allowed to be false-positive; It may be set to `true` even
    /// if no property actually changed, but a `false` value always guarantees that the previous
    /// properties are still valid.
    pub has_changed: bool,
}

fn size_per_element(space: UVec2, num_elems: u32, direction: Direction) -> UVec2 {
    // Avoid zero div. Whatever, shouldn't really matter.
    if num_elems == 0 {
        return space;
    }

    match direction {
        Direction::Row => {
            let width = space.x;
            let height = space.y / num_elems;

            UVec2::new(width, height)
        }
        Direction::Column => {
            let width = space.x / num_elems;
            let height = space.y;

            UVec2::new(width, height)
        }
    }
}

#[cfg(test)]
mod tests {

    use glam::UVec2;
    use image::ImageBuffer;

    use crate::primitive::Primitive;
    use crate::render::Text;
    use crate::style::{
        Bounds, Direction, Growth, Justify, Padding, Position, Size, SizeVec2, Style,
    };

    use super::{size_per_element, ComputedBounds, Key, LayoutTree};

    #[test]
    fn size_per_element_row() {
        let space = UVec2::splat(1000);
        let num_elems = 5;
        let direction = Direction::Row;

        let output = size_per_element(space, num_elems, direction);

        assert_eq!(output.x, 1000);
        assert_eq!(output.y, 200);
    }

    #[test]
    fn size_per_element_column() {
        let space = UVec2::splat(1000);
        let num_elems = 5;
        let direction = Direction::Column;

        let output = size_per_element(space, num_elems, direction);

        assert_eq!(output.x, 200);
        assert_eq!(output.y, 1000);
    }

    #[test]
    fn compute_layout_no_children() {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let style = Style::default();

        let elem = Primitive {
            style: style.clone(),
            image: Some(ImageBuffer::new(128, 128)),
            text: None,
        };

        let key0 = tree.push(None, elem.clone());
        let key1 = tree.push(None, elem.clone());

        tree.compute_layout();

        let layout0 = &tree.layouts[&key0];
        let layout1 = &tree.layouts[&key1];

        assert_eq!(layout0.position, UVec2::splat(0));
        assert_eq!(layout0.width, 128);
        assert_eq!(layout0.height, 128);

        assert_eq!(layout1.position, UVec2::new(0, layout0.height));
        assert_eq!(layout1.width, 128);
        assert_eq!(layout1.height, 128);
    }

    #[test]
    fn compute_bounds_container_growth() {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let elem = Primitive::from_style(Style {
            growth: Growth::splat(1.0),
            ..Default::default()
        });

        let key = tree.push(None, elem);
        let bounds = tree.compute_bounds(key);

        assert_eq!(
            bounds,
            ComputedBounds {
                min: UVec2::splat(0),
                max: UVec2::splat(u32::MAX),
            }
        );
    }

    #[test]
    fn compute_bounds_container_no_growth() {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let elem = Primitive::from_style(Style {
            growth: Growth::NONE,
            ..Default::default()
        });

        let key = tree.push(None, elem);
        let bounds = tree.compute_bounds(key);

        assert_eq!(
            bounds,
            ComputedBounds {
                min: UVec2::splat(0),
                max: UVec2::splat(0),
            }
        );
    }

    #[test]
    fn compute_bounds_container_growth_children() {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let style = Style {
            growth: Growth::splat(1.0),
            ..Default::default()
        };

        let root = Primitive::from_style(style.clone());
        let key = tree.push(None, root);

        let elem = Primitive {
            style: Style::default(),
            image: Some(ImageBuffer::new(128, 128)),
            text: None,
        };
        tree.push(Some(key), elem.clone());

        let bounds = tree.compute_bounds(key);

        assert_eq!(
            bounds,
            ComputedBounds {
                min: UVec2::splat(128),
                max: UVec2::splat(u32::MAX),
            }
        );
    }

    #[test]
    fn compute_bounds_container_no_growth_children() {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let style = Style {
            growth: Growth::NONE,
            ..Default::default()
        };

        let root = Primitive::from_style(style.clone());
        let key = tree.push(None, root);

        let elem = Primitive {
            style: Style::default(),
            image: Some(ImageBuffer::new(128, 128)),
            text: None,
        };
        tree.push(Some(key), elem.clone());

        let bounds = tree.compute_bounds(key);

        assert_eq!(
            bounds,
            ComputedBounds {
                min: UVec2::splat(128),
                max: UVec2::splat(128),
            }
        );
    }

    #[test]
    fn compute_bounds_ignores_absolute_position() {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let root = Primitive::from_style(Style {
            growth: Growth::NONE,
            ..Default::default()
        });
        let key = tree.push(None, root);

        let elem = Primitive {
            style: Style {
                position: Position::Absolute(UVec2::splat(0)),
                ..Default::default()
            },
            image: None,
            text: Some(Text::new("test", 100.0, None)),
        };
        tree.push(Some(key), elem.clone());

        let bounds = tree.compute_bounds(key);
        assert_eq!(bounds, ComputedBounds::ZERO);
    }

    #[test]
    fn compute_bounds_exact_size() {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let bounds = Bounds {
            min: SizeVec2::splat(Size::Pixels(10)),
            max: SizeVec2::splat(Size::Pixels(10)),
        };

        let key = tree.push(
            None,
            Primitive::from_style(Style {
                bounds,
                ..Default::default()
            }),
        );

        let bounds = tree.compute_bounds(key);

        assert_eq!(bounds.min, UVec2::splat(10));
        assert_eq!(bounds.max, UVec2::splat(10));
    }

    #[test]
    fn compute_bounds_nested_column() {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let root = tree.push(None, Primitive::default());

        for _ in 0..10 {
            let wrapper = tree.push(
                Some(root),
                Primitive::from_style(Style {
                    direction: Direction::Column,
                    ..Default::default()
                }),
            );

            tree.push(
                Some(wrapper),
                Primitive::from_style(Style {
                    bounds: Bounds::exact(SizeVec2::splat(Size::Pixels(15))),
                    ..Default::default()
                }),
            );
        }

        let bounds = tree.compute_bounds(root);

        assert_eq!(bounds.min, UVec2::new(15, 15 * 10));
        assert_eq!(bounds.max, UVec2::new(15, 15 * 10));
    }

    #[test]
    fn computed_bounds_padding_overflow() {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let root = tree.push(
            None,
            Primitive::from_style(Style {
                bounds: Bounds::exact(SizeVec2::splat(Size::Pixels(u32::MAX))),
                padding: Padding::splat(Size::Pixels(1)),
                ..Default::default()
            }),
        );

        let bounds = tree.compute_bounds(root);

        assert_eq!(bounds.min, UVec2::new(u32::MAX, u32::MAX));
        assert_eq!(bounds.max, UVec2::new(u32::MAX, u32::MAX));
    }

    #[test]
    fn compute_layout_exact_size_root() {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let bounds = Bounds {
            min: SizeVec2::splat(Size::Pixels(10)),
            max: SizeVec2::splat(Size::Pixels(10)),
        };

        let key = tree.push(
            None,
            Primitive::from_style(Style {
                bounds,
                ..Default::default()
            }),
        );

        tree.compute_layout();

        let layout = &tree.layouts[&key];
        assert_eq!(layout.height, 10);
        assert_eq!(layout.width, 10);
    }

    #[test]
    fn compute_layout_exact_size_children() {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let bounds = Bounds {
            min: SizeVec2::splat(Size::Pixels(10)),
            max: SizeVec2::splat(Size::Pixels(10)),
        };

        let root = tree.push(None, Primitive::default());

        let key = tree.push(
            Some(root),
            Primitive::from_style(Style {
                bounds,
                ..Default::default()
            }),
        );

        tree.compute_layout();

        let layout = &tree.layouts[&key];
        assert_eq!(layout.height, 10);
        assert_eq!(layout.width, 10);
    }

    fn create_justify_test(
        direction: Direction,
        justify: Justify,
        num_elems: u32,
        size: u32,
    ) -> (LayoutTree, Vec<Key>) {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let root = tree.push(
            None,
            Primitive::from_style(Style {
                // Claim entire viewport so we can actually test
                // child positions.
                bounds: Bounds {
                    min: SizeVec2::splat(Size::Pixels(1000)),
                    max: SizeVec2::splat(Size::Pixels(1000)),
                },
                direction,
                justify,
                ..Default::default()
            }),
        );

        let elem = Primitive::from_style(Style {
            bounds: Bounds {
                min: SizeVec2::splat(Size::Pixels(size)),
                max: SizeVec2::splat(Size::Pixels(size)),
            },
            ..Default::default()
        });

        let keys = (0..num_elems)
            .map(|_| tree.push(Some(root), elem.clone()))
            .collect();

        tree.compute_layout();

        (tree, keys)
    }

    #[test]
    fn compute_layout_row_justify_start() {
        let size = 10;

        let (tree, keys) = create_justify_test(Direction::Row, Justify::Start, 3, size);

        let mut offset = 0;
        for key in keys {
            let layout = &tree.layouts[&key];

            assert_eq!(layout.position, UVec2::new(0, offset));

            offset += size;
        }
    }

    #[test]
    fn compute_layout_column_justify_start() {
        let size = 10;

        let (tree, keys) = create_justify_test(Direction::Column, Justify::Start, 3, size);

        let mut offset = 0;
        for key in keys {
            let layout = &tree.layouts[&key];

            assert_eq!(layout.position, UVec2::new(offset, 0));

            offset += size;
        }
    }

    #[test]
    fn compute_layout_row_justify_end() {
        let size = 10;

        let (tree, keys) = create_justify_test(Direction::Row, Justify::End, 3, size);

        let mut offset = 1000 - (size * 3);
        for key in keys {
            let layout = &tree.layouts[&key];

            assert_eq!(layout.position, UVec2::new(0, offset));

            offset += size;
        }
    }

    #[test]
    fn compute_layout_column_justify_end() {
        let size = 10;

        let (tree, keys) = create_justify_test(Direction::Column, Justify::End, 3, size);

        let mut offset = 1000 - (size * 3);
        for key in keys {
            let layout = &tree.layouts[&key];

            assert_eq!(layout.position, UVec2::new(offset, 0));

            offset += size;
        }
    }

    #[test]
    fn compute_layout_row_justify_center() {
        let size = 10;

        let (tree, keys) = create_justify_test(Direction::Row, Justify::Center, 3, size);

        let mut offset = (1000 - (10 * 3)) / 2;
        for key in keys {
            let layout = &tree.layouts[&key];

            assert_eq!(layout.position, UVec2::new(0, offset));

            offset += size;
        }
    }

    #[test]
    fn compute_layout_column_justify_center() {
        let size = 10;

        let (tree, keys) = create_justify_test(Direction::Column, Justify::Center, 3, size);

        let mut offset = (1000 - (10 * 3)) / 2;
        for key in keys {
            let layout = &tree.layouts[&key];

            assert_eq!(layout.position, UVec2::new(offset, 0));

            offset += size;
        }
    }

    #[test]
    fn compute_layout_row_justify_space_between() {
        let size = 10;

        let (tree, keys) = create_justify_test(Direction::Row, Justify::SpaceBetween, 3, size);

        let distance = (1000 - (size * 3)) / 2;

        let mut offset = 0;
        for key in keys {
            let layout = &tree.layouts[&key];

            assert_eq!(layout.position, UVec2::new(0, offset));

            offset += size + distance;
        }
    }

    #[test]
    fn compute_layout_column_justify_space_between() {
        let size = 10;

        let (tree, keys) = create_justify_test(Direction::Column, Justify::SpaceBetween, 3, size);

        let distance = (1000 - (size * 3)) / 2;

        let mut offset = 0;
        for key in keys {
            let layout = &tree.layouts[&key];

            assert_eq!(layout.position, UVec2::new(offset, 0));

            offset += size + distance;
        }
    }

    #[test]
    fn compute_layout_row_justify_space_around() {
        let size = 10;

        let (tree, keys) = create_justify_test(Direction::Row, Justify::SpaceAround, 3, size);

        let distance = (1000 - (size * 3)) / 4;

        let mut offset = distance;
        for key in keys {
            let layout = &tree.layouts[&key];

            assert_eq!(layout.position, UVec2::new(0, offset));

            offset += size + distance;
        }
    }

    #[test]
    fn compute_layout_column_justify_space_around() {
        let size = 10;

        let (tree, keys) = create_justify_test(Direction::Column, Justify::SpaceAround, 3, size);

        let distance = (1000 - (size * 3)) / 4;

        let mut offset = distance;
        for key in keys {
            let layout = &tree.layouts[&key];

            assert_eq!(layout.position, UVec2::new(offset, 0));

            offset += size + distance;
        }
    }

    #[test]
    fn compute_layout_padding_no_children() {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let key = tree.push(
            None,
            Primitive::from_style(Style {
                bounds: Bounds {
                    min: SizeVec2::splat(Size::Pixels(10)),
                    max: SizeVec2::splat(Size::Pixels(10)),
                },
                padding: Padding::splat(Size::Pixels(10)),
                ..Default::default()
            }),
        );

        tree.compute_layout();

        let layout = &tree.layouts[&key];

        assert_eq!(layout.position, UVec2::splat(0));
    }

    #[test]
    fn computed_layout_padding_with_children() {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let root = tree.push(
            None,
            Primitive::from_style(Style {
                padding: Padding::splat(Size::Pixels(10)),
                ..Default::default()
            }),
        );

        let elem = Primitive::from_style(Style {
            bounds: Bounds {
                min: SizeVec2::splat(Size::Pixels(10)),
                max: SizeVec2::splat(Size::Pixels(10)),
            },
            ..Default::default()
        });

        let keys: Vec<_> = (0..3)
            .map(|_| tree.push(Some(root), elem.clone()))
            .collect();

        tree.compute_layout();

        let mut offset = UVec2::new(10, 10);
        for key in keys {
            let layout = &tree.layouts[&key];

            assert_eq!(layout.position, offset);

            offset.y += 10;
        }
    }

    #[test]
    fn compute_layout_ignore_absolute_children() {
        let mut tree = LayoutTree::new();
        tree.resize(UVec2::splat(1000));

        let root = tree.push(None, Primitive::default());

        let elem = Primitive::from_style(Style {
            bounds: Bounds::exact(SizeVec2::splat(Size::Pixels(10))),
            ..Default::default()
        });

        let mut keys: Vec<_> = (0..3)
            .map(|_| tree.push(Some(root), elem.clone()))
            .collect();

        tree.push(
            Some(root),
            Primitive::from_style(Style {
                bounds: Bounds::exact(SizeVec2::splat(Size::Pixels(10))),
                position: Position::Absolute(UVec2::splat(0)),
                ..Default::default()
            }),
        );

        keys.extend((0..3).map(|_| tree.push(Some(root), elem.clone())));

        tree.compute_layout();

        let mut offset = UVec2::splat(0);
        for key in keys {
            let layout = &tree.layouts[&key];

            assert_eq!(layout.position, offset);

            offset.y += 10;
        }
    }
}
