use glam::Vec2;

use super::style::{Bounds, Style};

#[derive(Clone, Debug)]
pub struct ComputedStyle {
    // FIXME: Should this exist on top of ComputedStyle?
    pub style: Style,
    pub bounds: ComputedBounds,
    pub padding: ComputedPadding,
    pub border_radius: ComputedBorderRadius,
}

impl ComputedStyle {
    pub fn new(style: Style, viewport: Vec2) -> Self {
        Self {
            bounds: ComputedBounds::default(),
            padding: ComputedPadding {
                top: style.padding.top.to_pixels(viewport),
                bottom: style.padding.bottom.to_pixels(viewport),
                left: style.padding.left.to_pixels(viewport),
                right: style.padding.right.to_pixels(viewport),
            },
            border_radius: ComputedBorderRadius {
                top_left: style.border_radius.top_left.to_pixels(viewport),
                bottom_left: style.border_radius.bottom_left.to_pixels(viewport),
                top_right: style.border_radius.top_right.to_pixels(viewport),
                bottom_right: style.border_radius.bottom_right.to_pixels(viewport),
            },
            style,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ComputedBounds {
    pub min: Vec2,
    pub max: Vec2,
}

impl ComputedBounds {
    pub const ZERO: Self = Self {
        min: Vec2::splat(0.0),
        max: Vec2::splat(0.0),
    };

    pub fn new(bounds: Bounds, viewport: Vec2) -> Self {
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

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct ComputedPadding {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct ComputedBorderRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_left: f32,
    pub bottom_right: f32,
}
