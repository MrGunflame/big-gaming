pub(crate) mod container;
pub(crate) mod debug;
pub mod image;
mod pipeline;
pub mod remap;
pub(crate) mod text;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use ::image::{ImageBuffer, Rgba};
use game_common::components::Color;
use game_render::camera::RenderTarget;
use game_render::graph::{NodeLabel, SlotFlags, SlotKind, SlotLabel};
use game_render::{Renderer, FINAL_RENDER_PASS};
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::RwLock;
use pipeline::Vertex;
use wgpu::Texture;

use crate::layout::{Key, Layout};
use crate::primitive::Primitive;

pub use self::image::Image;
use self::pipeline::UiPass;
pub use self::text::Text;

pub struct UiRenderer {
    targets: HashMap<RenderTarget, SurfaceState>,
    elements: Arc<RwLock<HashMap<RenderTarget, SurfaceDrawCommands>>>,
}

impl UiRenderer {
    pub fn new(renderer: &mut Renderer) -> Self {
        let device = renderer.device();

        let elements = Arc::new(RwLock::new(HashMap::new()));

        let ui_pass = UiPass::new(device, elements.clone());
        const UI_PASS: NodeLabel = NodeLabel::new("UI_PASS");

        let mut graph = renderer.graph_mut();
        graph.add_node(UI_PASS, ui_pass);
        graph.add_node_dependency(UI_PASS, FINAL_RENDER_PASS);
        graph.add_slot_dependency(
            UI_PASS,
            SlotLabel::SURFACE,
            SlotKind::Texture,
            SlotFlags::WRITE,
        );

        Self {
            targets: HashMap::new(),
            elements,
        }
    }

    pub fn insert(&mut self, target: RenderTarget, size: UVec2, scale_factor: f64) {
        self.targets.insert(
            target,
            SurfaceState {
                size,
                scale_factor,
                nodes: Vec::new(),
            },
        );

        let mut elems = self.elements.write();
        elems.insert(target, SurfaceDrawCommands::new());
    }

    pub(crate) fn get_mut(&mut self, target: RenderTarget) -> Option<&mut SurfaceState> {
        self.targets.get_mut(&target)
    }

    pub fn remove(&mut self, target: RenderTarget) {
        self.targets.remove(&target);

        let mut elems = self.elements.write();
        elems.remove(&target);
    }

    pub fn resize(&mut self, target: RenderTarget, size: UVec2) {
        tracing::trace!("resize {size}");
        if let Some(state) = self.targets.get_mut(&target) {
            state.size = size;
        }
    }

    pub fn update_scale_factor(&mut self, target: RenderTarget, scale_factor: f64) {
        if let Some(state) = self.targets.get_mut(&target) {
            state.scale_factor = scale_factor;
        }
    }

    pub fn update(&mut self) {
        let _span = trace_span!("UiRenderer::update").entered();

        for (id, state) in self.targets.iter_mut() {
            let mut surfaces = self.elements.write();
            let cmds = surfaces.get_mut(id).unwrap();

            cmds.begin_tracking();
            for (key, layout, elem) in &state.nodes {
                if !layout.has_changed {
                    cmds.track(*key);
                    continue;
                }

                let mut should_render = true;

                // Don't render elements with a zero size.
                if layout.width == 0 || layout.height == 0 {
                    should_render = false;
                }

                // Don't render elements that start outside of the viewport.
                if layout.position.x > state.size.x || layout.position.y > state.size.y {
                    should_render = false;
                }

                let cmd = if should_render {
                    elem.draw(
                        &layout.style,
                        Rect {
                            min: layout.position,
                            max: layout.position + UVec2::new(layout.width, layout.height),
                        },
                        state.size,
                        state.scale_factor,
                    )
                } else {
                    None
                };

                cmds.insert(*key, cmd);
            }

            cmds.finish_tracking();
        }
    }
}

#[derive(Clone, Debug)]
pub struct DrawCommand {
    pub position: Rect,
    pub color: Color,
    pub image: ImageBuffer<Rgba<u8>, Vec<u8>>,
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

#[derive(Debug)]
struct SurfaceDrawCommands {
    // Note: We use `None` to represent primitives that do
    // not need to be rendered. We can still retain them
    // over frames.
    cmds: BTreeMap<Key, Option<DrawCommandState>>,
    tracked: HashSet<Key>,
}

impl SurfaceDrawCommands {
    fn new() -> Self {
        Self {
            cmds: BTreeMap::new(),
            tracked: HashSet::new(),
        }
    }

    fn begin_tracking(&mut self) {
        self.tracked.clear();
    }

    fn finish_tracking(&mut self) {
        self.cmds.retain(|k, _| self.tracked.contains(k));
    }

    fn track(&mut self, key: Key) {
        self.tracked.insert(key);
    }

    fn insert(&mut self, key: Key, cmd: Option<DrawCommand>) {
        debug_assert!(!self.tracked.contains(&key));

        self.cmds.insert(
            key,
            cmd.map(|cmd| DrawCommandState {
                cmd,
                gpu_state: None,
            }),
        );
        self.tracked.insert(key);
    }

    fn commands_mut(&mut self) -> impl Iterator<Item = &mut DrawCommandState> + '_ {
        self.cmds.values_mut().filter_map(|v| v.as_mut())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SurfaceState {
    pub(crate) size: UVec2,
    pub(crate) scale_factor: f64,
    pub(crate) nodes: Vec<(Key, Layout, Primitive)>,
}

#[derive(Debug)]
struct DrawCommandState {
    cmd: DrawCommand,
    gpu_state: Option<GpuDrawCommandState>,
}

#[derive(Debug)]
struct GpuDrawCommandState {
    vertices: [Vertex; 4],
    texture: Texture,
    /// Viewport size for which this draw command is build.
    size: UVec2,
}
