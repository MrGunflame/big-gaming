pub(crate) mod container;
pub(crate) mod debug;
pub mod image;
mod pipeline;
pub mod remap;
pub(crate) mod text;

use std::collections::HashMap;
use std::sync::Arc;

use ::image::{ImageBuffer, Rgba};
use game_common::components::Color;
use game_render::camera::RenderTarget;
use game_render::Renderer;
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::RwLock;

use crate::layout::computed_style::ComputedStyle;
use crate::layout::LayoutTree;

pub use self::image::Image;
use self::pipeline::UiPass;
pub use self::text::Text;
pub use crate::layout::{Element, ElementBody};

pub struct UiRenderer {
    targets: HashMap<RenderTarget, LayoutTree>,
    elements: Arc<RwLock<HashMap<RenderTarget, Vec<DrawCommand>>>>,
}

impl UiRenderer {
    pub fn new(renderer: &Renderer) -> Self {
        let device = renderer.device();

        let elements = Arc::new(RwLock::new(HashMap::new()));

        renderer.add_to_graph(UiPass::new(device, elements.clone()));

        Self {
            targets: HashMap::new(),
            elements,
        }
    }

    pub fn insert(&mut self, target: RenderTarget, size: UVec2) {
        self.targets.insert(target, LayoutTree::new());
        self.resize(target, size);

        let mut elems = self.elements.write();
        elems.insert(target, vec![]);
    }

    pub fn get_mut(&mut self, target: RenderTarget) -> Option<&mut LayoutTree> {
        self.targets.get_mut(&target)
    }

    pub fn remove(&mut self, target: RenderTarget) {
        self.targets.remove(&target);

        let mut elems = self.elements.write();
        elems.remove(&target);
    }

    pub fn resize(&mut self, target: RenderTarget, size: UVec2) {
        if let Some(tree) = self.targets.get_mut(&target) {
            tree.resize(size);
        }
    }

    pub fn update(&mut self) {
        let _span = trace_span!("UiRenderer::update").entered();

        for (id, tree) in self.targets.iter_mut() {
            tree.compute_layout();

            let size = tree.size();

            let mut cmds = vec![];
            for (elem, layout) in tree.elements().zip(tree.layouts()) {
                // Don't render elements with a zero size.
                if layout.width == 0 || layout.height == 0 {
                    continue;
                }

                // Don't render elements that start outside of the viewport.
                if layout.position.x > size.x || layout.position.y > size.y {
                    continue;
                }

                if let Some(cmd) = elem.draw(
                    &layout.style,
                    Rect {
                        min: layout.position,
                        max: layout.position + UVec2::new(layout.width, layout.height),
                    },
                    size,
                ) {
                    cmds.push(cmd);
                }
            }

            *self.elements.write().get_mut(id).unwrap() = cmds;
        }
    }
}

#[derive(Clone, Debug)]
pub struct DrawCommand {
    pub position: Rect,
    pub color: Color,
    pub image: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

pub(crate) trait DrawElement {
    fn draw(&self, style: &ComputedStyle, layout: Rect, size: UVec2) -> Option<DrawCommand>;
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect {
    pub min: UVec2,
    pub max: UVec2,
}

impl Rect {
    #[inline]
    pub fn size(self) -> UVec2 {
        self.max - self.min
    }

    #[inline]
    pub fn width(self) -> u32 {
        self.max.x - self.min.x
    }

    #[inline]
    pub fn height(self) -> u32 {
        self.max.y - self.min.y
    }

    pub fn contains(self, point: UVec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }
}
