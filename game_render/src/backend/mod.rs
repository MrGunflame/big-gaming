use std::num::NonZeroU32;

use ash::vk;
use bitflags::bitflags;
use glam::UVec2;
use vulkan::{ShaderModule, TextureView};

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
    pub min_images: u32,
    pub max_images: Option<NonZeroU32>,
    pub formats: Vec<TextureFormat>,
    pub present_modes: Vec<PresentMode>,
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
}

#[derive(Clone, Debug)]
pub struct SwapchainConfig {
    pub image_count: u32,
    pub extent: UVec2,
    pub format: TextureFormat,
    pub present_mode: PresentMode,
}

pub struct PipelineDescriptor<'a> {
    pub stages: &'a [PipelineStage<'a>],
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
}

pub struct RenderPassDescriptor<'a> {
    pub color_attachments: &'a [RenderPassColorAttachment<'a>],
}

pub struct RenderPassColorAttachment<'a> {
    pub view: &'a TextureView<'a>,
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
