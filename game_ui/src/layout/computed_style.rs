use glam::UVec2;

use crate::style::{Bounds, Style};

#[derive(Clone, Debug)]
pub struct ComputedStyle {
    // FIXME: Should this exist on top of ComputedStyle?
    pub style: Style,
    pub bounds: ComputedBounds,
    pub padding: ComputedPadding,
    pub border_radius: ComputedBorderRadius,
    pub border: ComputedBorder,
}

impl ComputedStyle {
    pub fn new(style: Style, viewport: UVec2, scale_factor: f32) -> Self {
        Self {
            bounds: ComputedBounds::default(),
            padding: ComputedPadding {
                top: style.padding.top.to_pixels(viewport, scale_factor),
                bottom: style.padding.bottom.to_pixels(viewport, scale_factor),
                left: style.padding.left.to_pixels(viewport, scale_factor),
                right: style.padding.right.to_pixels(viewport, scale_factor),
            },
            border_radius: ComputedBorderRadius {
                top_left: style
                    .border_radius
                    .top_left
                    .to_pixels(viewport, scale_factor),
                bottom_left: style
                    .border_radius
                    .bottom_left
                    .to_pixels(viewport, scale_factor),
                top_right: style
                    .border_radius
                    .top_right
                    .to_pixels(viewport, scale_factor),
                bottom_right: style
                    .border_radius
                    .bottom_right
                    .to_pixels(viewport, scale_factor),
            },
            border: ComputedBorder {
                top: style.border.top.to_pixels(viewport, scale_factor),
                bottom: style.border.bottom.to_pixels(viewport, scale_factor),
                left: style.border.left.to_pixels(viewport, scale_factor),
                right: style.border.right.to_pixels(viewport, scale_factor),
            },
            style,
        }
    }

    pub(super) fn equal_except_style(&self, other: &Self) -> bool {
        self.bounds == other.bounds
            && self.padding == other.padding
            && self.border_radius == other.border_radius
            && self.border == other.border
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ComputedBounds {
    pub min: UVec2,
    pub max: UVec2,
}

impl ComputedBounds {
    pub const ZERO: Self = Self {
        min: UVec2::ZERO,
        max: UVec2::ZERO,
    };

    pub fn new(bounds: Bounds, viewport: UVec2, scale_factor: f32) -> Self {
        Self {
            min: UVec2 {
                x: bounds.min.x.to_pixels(viewport, scale_factor),
                y: bounds.min.y.to_pixels(viewport, scale_factor),
            },
            max: UVec2 {
                x: bounds.max.x.to_pixels(viewport, scale_factor),
                y: bounds.max.y.to_pixels(viewport, scale_factor),
            },
        }
    }
}

impl Default for ComputedBounds {
    fn default() -> Self {
        Self {
            min: UVec2::ZERO,
            max: UVec2::MAX,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ComputedPadding {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ComputedBorderRadius {
    pub top_left: u32,
    pub top_right: u32,
    pub bottom_left: u32,
    pub bottom_right: u32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ComputedBorder {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}
