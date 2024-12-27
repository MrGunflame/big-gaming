use std::num::{NonZeroU32, NonZeroU64};

use ash::vk;
use bitflags::bitflags;
use glam::UVec2;
use vulkan::{ShaderModule, TextureView};

pub mod allocator;
pub mod vulkan;

#[derive(Clone, Debug)]
pub struct AdapterProperties {
    pub name: String,
    pub kind: AdapterKind,
}

#[derive(Copy, Clone, Debug)]
pub enum AdapterKind {
    DiscreteGpu,
    IntegratedGpu,
    Other,
}

#[derive(Clone, Debug)]
pub struct AdapterMemoryProperties {
    pub heaps: Vec<MemoryHeap>,
    pub types: Vec<MemoryType>,
}

#[derive(Copy, Clone, Debug)]
pub struct MemoryHeap {
    pub id: u32,
    pub size: u64,
}

#[derive(Copy, Clone, Debug)]
pub struct MemoryType {
    pub id: u32,
    pub heap: u32,
    pub flags: MemoryTypeFlags,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct MemoryTypeFlags: u32 {
        /// Memory that is local the GPU.
        const DEVICE_LOCAL = 1 << 0;
        /// Memory that can be accessed by the CPU after mapping.
        const HOST_VISIBLE = 1 << 1;
        /// If not set the CPU needs to flush the host cache.
        const HOST_COHERENT = 1 << 2;
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct QueueCapabilities: u32 {
        const GRAPHICS = 1 << 0;
        const COMPUTE = 1 << 1;
        const TRANSFER = 1 << 2;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct QueueFamily {
    pub id: u32,
    pub count: u32,
    pub capabilities: QueueCapabilities,
}

#[derive(Clone, Debug)]
pub struct SwapchainCapabilities {
    pub min_extent: UVec2,
    pub max_extent: UVec2,
    /// The minimum number of images that must be in a swapchain.
    pub min_images: u32,
    /// The maximum number of images that can be in a swapchain. `None` indicates no limit.
    pub max_images: Option<NonZeroU32>,
    pub formats: Vec<TextureFormat>,
    pub present_modes: Vec<PresentMode>,
    current_transform: vk::SurfaceTransformFlagsKHR,
    supported_composite_alpha: vk::CompositeAlphaFlagsKHR,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PresentMode {
    Fifo,
    Immediate,
    FifoRelaxed,
    Mailbox,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    R8G8B8A8Unorm,
    R8G8B8A8UnormSrgb,
    B8G8R8A8Unorm,
    B8G8R8A8UnormSrgb,
}

#[derive(Clone, Debug)]
pub struct SwapchainConfig {
    pub image_count: u32,
    pub extent: UVec2,
    pub format: TextureFormat,
    pub present_mode: PresentMode,
}

pub struct PipelineDescriptor<'a> {
    pub topology: PrimitiveTopology,
    pub front_face: FrontFace,
    pub cull_mode: Option<Face>,
    pub stages: &'a [PipelineStage<'a>],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveTopology {
    TriangleList,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FrontFace {
    Cw,
    Ccw,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Front,
    Back,
}

pub enum PipelineStage<'a> {
    Vertex(VertexStage<'a>),
    Fragment(FragmentStage<'a>),
}

pub struct VertexStage<'a> {
    pub shader: &'a ShaderModule<'a>,
}

pub struct FragmentStage<'a> {
    pub shader: &'a ShaderModule<'a>,
    pub targets: &'a [TextureFormat],
}

pub struct RenderPassDescriptor<'a, 'res> {
    pub color_attachments: &'a [RenderPassColorAttachment<'res>],
}

pub struct RenderPassColorAttachment<'res> {
    pub view: &'res TextureView<'res>,
    pub layout: vk::ImageLayout,
    pub size: UVec2,
    pub load_op: LoadOp,
    pub store_op: StoreOp,
}

pub enum LoadOp {
    Clear([f32; 4]),
    Load,
}

pub enum StoreOp {
    Discard,
    Store,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct BufferUsage: u32 {
        const TRANSFER_SRC = 1 << 0;
        const TRANSFER_DST = 1 << 1;
        const UNIFORM = 1 << 2;
        const STORAGE = 1 << 3;
        const VERTEX = 1 << 4;
        const INDEX = 1 << 5;
        const INDIRECT = 1 << 6;
    }
}

#[derive(Clone, Debug)]
pub struct MemoryRequirements {
    pub size: NonZeroU64,
    pub align: NonZeroU64,
    pub memory_types: Vec<u32>,
}

impl MemoryRequirements {
    pub fn padding_needed(&self) -> u64 {
        let size = self.size.get();
        let align = self.align.get();
        if size % align == 0 {
            0
        } else {
            align - (size % align)
        }
    }
}
