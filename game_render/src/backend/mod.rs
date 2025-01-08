pub mod allocator;
pub mod descriptors;
pub mod shader;
pub mod vulkan;

use std::num::{NonZeroU32, NonZeroU64};
use std::ops::Range;

use ash::vk::{self, PipelineStageFlags};
use bitflags::bitflags;
use glam::UVec2;
use thiserror::Error;
use vulkan::{Buffer, DescriptorSetLayout, Sampler, Semaphore, ShaderModule, TextureView};

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
    pub descriptors: &'a [&'a DescriptorSetLayout],
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
    pub shader: &'a ShaderModule,
}

pub struct FragmentStage<'a> {
    pub shader: &'a ShaderModule,
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

#[derive(Copy, Clone, Debug)]
pub enum LoadOp {
    Clear([f32; 4]),
    Load,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum StoreOp {
    Discard,
    Store,
}

// FIXME: Maybe not the best name.
#[derive(Copy, Clone, Debug)]
pub struct DescriptorSetDescriptor<'a> {
    pub bindings: &'a [DescriptorBinding],
}

#[derive(Copy, Clone, Debug)]
pub struct DescriptorBinding {
    pub binding: u32,
    pub visibility: ShaderStages,
    pub kind: DescriptorType,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ShaderStages: u32 {
        const VERTEX = 1 << 0;
        const FRAGMENT = 1 << 1;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DescriptorType {
    Uniform,
    Storage,
    Sampler,
    Texture,
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

// TODO: Naming
#[derive(Clone, Debug)]
pub struct DescriptorPoolDescriptor {
    /// The maximum number of descriptor set that can be allocated from the pool.
    pub max_sets: NonZeroU32,
    /// The maximum number of combined uniform buffers that can be created in sets allocated in the
    /// pool.
    pub max_uniform_buffers: u32,
    pub max_storage_buffers: u32,
    pub max_samplers: u32,
    pub max_sampled_images: u32,
}

pub struct WriteDescriptorResources<'a> {
    pub bindings: &'a [WriteDescriptorBinding<'a>],
}

pub struct WriteDescriptorBinding<'a> {
    pub binding: u32,
    pub resource: WriteDescriptorResource<'a>,
}

pub enum WriteDescriptorResource<'a> {
    Buffer(&'a BufferView<'a>),
    Texture(&'a TextureView<'a>),
    Sampler(&'a Sampler),
}

#[derive(Clone, Debug)]
pub struct BufferView<'a> {
    buffer: &'a Buffer,
    view: Range<u64>,
}

impl<'a> BufferView<'a> {
    pub fn buffer(&self) -> &Buffer {
        self.buffer
    }

    pub fn offset(&self) -> u64 {
        self.view.start
    }

    pub fn len(&self) -> u64 {
        self.view.end - self.view.start
    }
}

#[derive(Clone, Debug)]
pub struct TextureDescriptor {
    pub size: UVec2,
    pub mip_levels: u32,
    pub format: TextureFormat,
}

impl TextureDescriptor {
    pub fn compute_size(&self) -> u64 {
        // TODO: Implement mips > 1
        assert_eq!(
            self.mip_levels, 1,
            "compute_size is not yet implemented for mips > 1"
        );

        let bytes_per_texel = match self.format {
            TextureFormat::R8G8B8A8Unorm => 4,
            TextureFormat::R8G8B8A8UnormSrgb => 4,
            TextureFormat::B8G8R8A8Unorm => 4,
            TextureFormat::B8G8R8A8UnormSrgb => 4,
        };

        bytes_per_texel * u64::from(self.size.x) * u64::from(self.size.y)
    }
}

pub struct PipelineBarriers<'a> {
    pub buffer: &'a [BufferBarrier<'a>],
    pub texture: &'a [TextureBarrier<'a>],
}

#[derive(Debug)]
pub struct BufferBarrier<'a> {
    pub buffer: &'a Buffer,
    pub offset: u64,
    pub size: u64,
    pub src_access: AccessFlags,
    pub dst_access: AccessFlags,
}

#[derive(Debug)]
pub struct TextureBarrier<'a> {
    pub texture: &'a vulkan::Texture,
    pub src_access: AccessFlags,
    pub dst_access: AccessFlags,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct AccessFlags: u32 {
        /// Resource can be used as a destination for transfer operations.
        const TRANSFER_WRITE = 1 << 0;
        /// Resource can be bound as readable in a shader module.
        const SHADER_READ = 1 << 1;
        /// Resource can be bound as a writable color attachment.
        const COLOR_ATTACHMENT_WRITE = 1 << 2;
        /// Resource can be used to present to the swapchain.
        const PRESENT = 1 << 3;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TextureLayout {
    Undefined,
    Present,
    ColorAttachment,
    TransferDst,
    ShaderRead,
}

#[derive(Debug)]
pub struct CopyBuffer<'a> {
    pub buffer: &'a Buffer,
    pub offset: u64,
    pub layout: ImageDataLayout,
}

#[derive(Copy, Clone, Debug)]
pub struct ImageDataLayout {
    pub bytes_per_row: u32,
    pub rows_per_image: u32,
}

#[derive(Debug)]
pub struct QueueSubmit<'a> {
    pub wait: &'a mut [&'a mut Semaphore],
    pub wait_stage: PipelineStageFlags,
    pub signal: &'a mut [&'a mut Semaphore],
}

pub struct SamplerDescriptor {
    pub min_filter: FilterMode,
    pub mag_filter: FilterMode,
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub address_mode_w: AddressMode,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FilterMode {
    Nearest,
    Linear,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AddressMode {
    Repeat,
    MirrorRepeat,
    ClampToEdge,
    ClampToBorder,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum IndexFormat {
    U16,
    U32,
}
