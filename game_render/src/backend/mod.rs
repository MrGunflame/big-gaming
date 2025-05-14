pub mod allocator;
pub mod descriptors;
pub mod vulkan;

use std::num::{NonZeroU32, NonZeroU64};
use std::ops::Range;

use ash::vk;
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use game_common::components::Color;
use glam::UVec2;
use hashbrown::HashMap;
use vulkan::{Buffer, DescriptorSetLayout, Fence, Sampler, Semaphore, TextureView};

use crate::shader::Shader;

#[derive(Clone, Debug)]
pub struct AdapterProperties {
    pub name: String,
    pub kind: AdapterKind,
    /// What operations are supported by which texture formats.
    pub formats: HashMap<TextureFormat, TextureUsage>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AdapterKind {
    DiscreteGpu,
    IntegratedGpu,
    /// Software rasterizer
    Cpu,
    Other,
}

#[derive(Clone, Debug)]
pub struct AdapterMemoryProperties {
    pub heaps: Vec<MemoryHeap>,
    pub types: Vec<MemoryType>,
    /// Maximum size of a single allocation.
    pub max_allocation_size: NonZeroU64,
}

#[derive(Copy, Clone, Debug)]
pub struct MemoryHeap {
    pub id: u32,
    pub size: u64,
    pub flags: MemoryHeapFlags,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct MemoryHeapFlags: u8 {
        const DEVICE_LOCAL = 1 << 0;
    }

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
        const _VK_PROTECTED = 1 << 3;
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
    pub id: QueueFamilyId,
    pub count: u32,
    pub capabilities: QueueCapabilities,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct QueueFamilyId(u32);

#[derive(Clone, Debug)]
pub struct SwapchainCapabilities {
    pub min_extent: UVec2,
    pub max_extent: UVec2,
    /// The minimum number of images that must be in a swapchain.
    pub min_images: u32,
    /// The maximum number of images that can be in a swapchain. `None` indicates no limit.
    pub max_images: Option<NonZeroU32>,
    pub formats: Vec<SurfaceFormat>,
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
    Rgba8Unorm,
    Rgba8UnormSrgb,
    Bgra8Unorm,
    Bgra8UnormSrgb,
    Depth32Float,
    Rgba16Float,
    Bc1RgbaUnorm,
    Bc1RgbaUnormSrgb,
    Bc2RgbaUnorm,
    Bc2RgbaUnormSrgb,
    Bc3RgbaUnorm,
    Bc3RgbaUnormSrgb,
    Bc4RUnorm,
    Bc4RSnorm,
    Bc5RgUnorm,
    Bc5RgSnorm,
    Bc6HRgbUFloat,
    Bc6HRgbSFloat,
    Bc7RgbaUnorm,
    Bc7RgbaUnormSrgb,
}

impl TextureFormat {
    pub const fn is_srgb(&self) -> bool {
        match self {
            Self::Rgba8UnormSrgb => true,
            Self::Bgra8UnormSrgb => true,
            Self::Bc1RgbaUnormSrgb => true,
            Self::Bc2RgbaUnormSrgb => true,
            Self::Bc3RgbaUnormSrgb => true,
            Self::Bc7RgbaUnormSrgb => true,
            _ => false,
        }
    }

    pub const fn is_depth(&self) -> bool {
        matches!(self, Self::Depth32Float)
    }

    /// Returns the number of bytes per 4x4 block.
    pub const fn bytes_per_block(&self) -> u32 {
        match self {
            // 8-bit
            TextureFormat::Rgba8Unorm => 64,
            TextureFormat::Rgba8UnormSrgb => 64,
            TextureFormat::Bgra8Unorm => 64,
            TextureFormat::Bgra8UnormSrgb => 64,
            TextureFormat::Rgba16Float => 128,
            TextureFormat::Depth32Float => 64,
            TextureFormat::Bc1RgbaUnorm => 8,
            TextureFormat::Bc1RgbaUnormSrgb => 8,
            TextureFormat::Bc2RgbaUnorm => 16,
            TextureFormat::Bc2RgbaUnormSrgb => 16,
            TextureFormat::Bc3RgbaUnorm => 16,
            TextureFormat::Bc3RgbaUnormSrgb => 16,
            TextureFormat::Bc4RUnorm => 8,
            TextureFormat::Bc4RSnorm => 8,
            TextureFormat::Bc5RgSnorm => 16,
            TextureFormat::Bc5RgUnorm => 16,
            TextureFormat::Bc6HRgbUFloat => 16,
            TextureFormat::Bc6HRgbSFloat => 16,
            TextureFormat::Bc7RgbaUnorm => 16,
            TextureFormat::Bc7RgbaUnormSrgb => 16,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Rgba8Unorm,
            Self::Rgba8UnormSrgb,
            Self::Bgra8Unorm,
            Self::Bgra8UnormSrgb,
            Self::Depth32Float,
            Self::Rgba16Float,
            Self::Bc1RgbaUnorm,
            Self::Bc1RgbaUnormSrgb,
            Self::Bc2RgbaUnorm,
            Self::Bc2RgbaUnormSrgb,
            Self::Bc3RgbaUnorm,
            Self::Bc3RgbaUnormSrgb,
            Self::Bc5RgUnorm,
            Self::Bc5RgSnorm,
        ]
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SwapchainConfig {
    pub image_count: u32,
    pub extent: UVec2,
    pub format: SurfaceFormat,
    pub present_mode: PresentMode,
}

pub struct PipelineDescriptor<'a> {
    pub topology: PrimitiveTopology,
    pub front_face: FrontFace,
    pub cull_mode: Option<Face>,
    pub stages: &'a [PipelineStage<'a>],
    pub descriptors: &'a [&'a DescriptorSetLayout],
    pub push_constant_ranges: &'a [PushConstantRange],
    pub depth_stencil_state: Option<DepthStencilState>,
}

#[derive(Copy, Clone, Debug)]
pub struct DepthStencilState {
    pub format: TextureFormat,
    pub depth_write_enabled: bool,
    pub depth_compare_op: CompareOp,
}

#[derive(Clone, Debug)]
pub struct PushConstantRange {
    pub range: Range<u32>,
    pub stages: ShaderStages,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PrimitiveTopology {
    TriangleList,
    LineList,
    PointList,
    LineStrip,
    TriangleStrip,
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

#[derive(Debug)]
pub enum PipelineStage<'a> {
    Vertex(VertexStage<'a>),
    Fragment(FragmentStage<'a>),
    Task(TaskStage<'a>),
    Mesh(MeshStage<'a>),
}

impl PipelineStage<'_> {
    fn shader_stage(&self) -> ShaderStage {
        match self {
            Self::Vertex(_) => ShaderStage::Vertex,
            Self::Fragment(_) => ShaderStage::Fragment,
            Self::Task(_) => ShaderStage::Task,
            Self::Mesh(_) => ShaderStage::Mesh,
        }
    }
}

#[derive(Debug)]
pub struct VertexStage<'a> {
    pub shader: &'a Shader,
    pub entry: &'static str,
}

#[derive(Debug)]
pub struct FragmentStage<'a> {
    pub shader: &'a Shader,
    pub entry: &'static str,
    pub targets: &'a [ColorTargetState],
}

#[derive(Debug)]
pub struct TaskStage<'a> {
    pub shader: &'a Shader,
    pub entry: &'static str,
}

#[derive(Debug)]
pub struct MeshStage<'a> {
    pub shader: &'a Shader,
    pub entry: &'static str,
}

#[derive(Clone, Debug)]
pub struct ColorTargetState {
    pub format: TextureFormat,
    pub blend: Option<BlendState>,
}

#[derive(Copy, Clone, Debug)]
pub struct BlendState {
    pub color_src_factor: BlendFactor,
    pub color_dst_factor: BlendFactor,
    pub color_op: BlendOp,
    pub alpha_src_factor: BlendFactor,
    pub alpha_dst_factor: BlendFactor,
    pub alpha_op: BlendOp,
}

impl BlendState {
    pub const PREMULTIPLIED_ALPHA: Self = Self {
        color_src_factor: BlendFactor::One,
        color_dst_factor: BlendFactor::OneMinusSrcAlpha,
        color_op: BlendOp::Add,
        alpha_src_factor: BlendFactor::One,
        alpha_dst_factor: BlendFactor::OneMinusSrcAlpha,
        alpha_op: BlendOp::Add,
    };

    pub const NON_PREMULTIPLIED_ALPHA: Self = Self {
        color_src_factor: BlendFactor::SrcAlpha,
        color_dst_factor: BlendFactor::OneMinusSrcAlpha,
        color_op: BlendOp::Add,
        alpha_src_factor: BlendFactor::One,
        alpha_dst_factor: BlendFactor::OneMinusSrcAlpha,
        alpha_op: BlendOp::Add,
    };
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlendFactor {
    Zero,
    One,
    Src,
    OneMinusSrc,
    SrcAlpha,
    OneMinusSrcAlpha,
    Dst,
    OneMinusDst,
    DstAlpha,
    OneMinusDstAlpha,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlendOp {
    Add,
}

#[derive(Debug)]
pub struct RenderPassDescriptor<'a, 'res> {
    pub color_attachments: &'a [RenderPassColorAttachment<'res>],
    pub depth_stencil_attachment: Option<&'a RenderPassDepthStencilAttachment<'res>>,
}

#[derive(Debug)]
pub struct RenderPassColorAttachment<'res> {
    pub view: &'res TextureView<'res>,
    pub load_op: LoadOp<Color>,
    pub store_op: StoreOp,
    pub access: AccessFlags,
}

#[derive(Debug)]
pub struct RenderPassDepthStencilAttachment<'res> {
    pub view: &'res TextureView<'res>,
    pub depth_load_op: LoadOp<f32>,
    pub depth_store_op: StoreOp,
    pub access: AccessFlags,
}

#[derive(Copy, Clone, Debug)]
pub enum CompareOp {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

#[derive(Copy, Clone, Debug)]
pub enum LoadOp<T> {
    Clear(T),
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
    pub count: NonZeroU32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Task,
    Mesh,
}

impl ShaderStage {
    pub const fn stages(self) -> ShaderStages {
        match self {
            Self::Vertex => ShaderStages::VERTEX,
            Self::Fragment => ShaderStages::FRAGMENT,
            Self::Task => ShaderStages::TASK,
            Self::Mesh => ShaderStages::MESH,
        }
    }
}

impl From<ShaderStage> for ShaderStages {
    fn from(value: ShaderStage) -> Self {
        value.stages()
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ShaderStages: u32 {
        const VERTEX = 1 << 0;
        const FRAGMENT = 1 << 1;
        const TASK = 1 << 2;
        const MESH = 1 << 3;
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

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct TextureUsage: u32 {
        /// The texture can be used as a copy source.
        const TRANSFER_SRC = 1 << 0;
        /// The texture can be used as a copy destination.
        const TRANSFER_DST = 1 << 1;
        /// The texture can be used as a target attachment in a render pass.
        const RENDER_ATTACHMENT = 1 << 2;
        /// The texture can be bound in a shader.
        const TEXTURE_BINDING = 1 << 3;
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

#[derive(Copy, Clone, Debug)]
pub enum WriteDescriptorResource<'a> {
    UniformBuffer(&'a [BufferView<'a>]),
    StorageBuffer(&'a [BufferView<'a>]),
    Texture(&'a [TextureView<'a>]),
    Sampler(&'a [Sampler]),
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
    pub usage: TextureUsage,
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
    pub base_mip_level: u32,
    pub mip_levels: u32,
}

bitflags! {
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct AccessFlags: u32 {
        /// Resource can be used as a source for transfer operations.
        const TRANSFER_READ = 1 << 0;
        /// Resource can be used as a destination for transfer operations.
        const TRANSFER_WRITE = 1 << 1;
        const COLOR_ATTACHMENT_READ = 1 << 2;
        /// Resource can be bound as a writable color attachment.
        const COLOR_ATTACHMENT_WRITE = 1 << 3;
        /// Resource can be used to present to the swapchain.
        const PRESENT = 1 << 4;
        /// Resources can be bound as a read-only index buffer.
        const INDEX = 1 << 5;
        /// Resource can be used as the source of an indirect command.
        const INDIRECT = 1 << 6;
        const DEPTH_ATTACHMENT_WRITE = 1 << 7;
        const DEPTH_ATTACHMENT_READ = 1 << 8;
        /// Resource can be bound and accessed readable in a vertex shader.
        const VERTEX_SHADER_READ = 1 << 9;
        /// Resource can be bound and accessed writable in a vertex shader.
        const VERTEX_SHADER_WRITE = 1 << 10;
        /// Resource can be bound and accessed readable in a fragment shader.
        const FRAGMENT_SHADER_READ = 1 << 11;
        /// Resource can be bound and accessed writable in a fragment shader.
        const FRAGMENT_SHADER_WRITE = 1 << 12;
        /// Resource can be bound and accessed reable in task shader.
        const TASK_SHADER_READ = 1 << 13;
        /// Resource can be bound and accessed writable in a task shader.
        const TASK_SHADER_WRITE = 1 << 14;
        /// Resource can be read from in a mesh shader.
        const MESH_SHADER_READ = 1 << 15;
        /// Resource can be written to in a mesh shader.
        const MESH_SHADER_WRITE = 1 << 16;
    }
}

impl AccessFlags {
    /// Resource can be bound and accessed reable in any shader.
    pub const SHADER_READ: Self =
        Self::from_bits(Self::VERTEX_SHADER_READ.bits() | Self::FRAGMENT_SHADER_READ.bits())
            .unwrap();

    pub const SHADER_WRITE: Self =
        Self::from_bits(Self::VERTEX_SHADER_WRITE.bits() | Self::FRAGMENT_SHADER_WRITE.bits())
            .unwrap();

    /// Returns `true` if all access flags are allowed to be used in a queue with the given
    /// [`QueueCapabilities`].
    fn is_allowed_for_queue(&self, caps: &QueueCapabilities) -> bool {
        // See https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#VUID-vkCmdPipelineBarrier2-dstStageMask-09676
        let graphics_flags = AccessFlags::COLOR_ATTACHMENT_READ
            | AccessFlags::COLOR_ATTACHMENT_WRITE
            | AccessFlags::INDEX
            | AccessFlags::INDIRECT
            | AccessFlags::DEPTH_ATTACHMENT_READ
            | AccessFlags::DEPTH_ATTACHMENT_WRITE
            | AccessFlags::VERTEX_SHADER_READ
            | AccessFlags::VERTEX_SHADER_WRITE
            | AccessFlags::FRAGMENT_SHADER_READ
            | AccessFlags::FRAGMENT_SHADER_WRITE
            | AccessFlags::TASK_SHADER_READ
            | AccessFlags::TASK_SHADER_WRITE
            | AccessFlags::MESH_SHADER_READ
            | AccessFlags::MESH_SHADER_WRITE;
        let compute_flags = AccessFlags::empty();
        let transfer_flags = AccessFlags::TRANSFER_READ | AccessFlags::TRANSFER_WRITE;

        let mut flags = *self;

        // The PRESENT flag is different in that it only defines a image
        // transition and no vulkan access flags.
        // Therefore there are no direct requirements on which queue this can
        // appear in a barrier.
        flags &= !AccessFlags::PRESENT;

        for cap in caps.iter() {
            match cap {
                QueueCapabilities::GRAPHICS => flags &= !graphics_flags,
                QueueCapabilities::COMPUTE => flags &= !compute_flags,
                QueueCapabilities::TRANSFER => flags &= !transfer_flags,
                _ => (),
            }
        }

        flags.is_empty()
    }
}

impl AccessFlags {
    /// Returns `true` if the flags specify only read-only operations.
    ///
    /// An empty set of flags is also classified as read-only.
    pub(crate) fn is_read_only(&self) -> bool {
        !self.is_writable()
    }

    pub(crate) fn is_writable(&self) -> bool {
        self.intersects(
            Self::TRANSFER_WRITE
                | Self::COLOR_ATTACHMENT_WRITE
                | Self::DEPTH_ATTACHMENT_WRITE
                | Self::VERTEX_SHADER_WRITE
                | Self::FRAGMENT_SHADER_WRITE
                | Self::MESH_SHADER_WRITE
                | Self::TASK_SHADER_WRITE,
        )
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
    /// Semaphores which will all be waited upon before the submit happens.
    pub wait: &'a mut [Semaphore],
    /// Semaphores which will be signaled once all commands in this submission have completed.
    ///
    /// All resources that have been used by commands in the submit command will no longer be
    /// accessed after any of the semaphores are signaled.
    pub signal: &'a mut [Semaphore],
    /// Fence to signal once all commands in this submission have completed.
    pub signal_fence: &'a mut Fence,
}

#[derive(Debug)]
pub struct QueuePresent<'a> {
    /// Semaphore that will be awaited before the presentation happens.
    pub wait: &'a mut Semaphore,
    /// Fence to signal once the presentation has been completed.
    pub signal: Option<&'a mut Fence>,
}

pub struct SamplerDescriptor {
    pub min_filter: FilterMode,
    pub mag_filter: FilterMode,
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub address_mode_w: AddressMode,
    pub mipmap_filter: FilterMode,
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

impl IndexFormat {
    /// Returns the number of bytes needed to represent an index.
    pub const fn size(&self) -> u8 {
        match self {
            Self::U16 => 2,
            Self::U32 => 4,
        }
    }
}

pub struct ShaderModuleDescriptor {}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct DrawIndirectArgs {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

impl DrawIndirectArgs {
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BarrierPipelineStage {
    /// ALL
    Top,
    /// TRANSFR/COMPUTE/GRAPHICS
    DrawIndirect,
    /// TRANSFER
    Transfer,
    /// COMPUTE
    Compute,
    /// GRAPHICS
    VertexInput,
    /// GRAPHICS
    VertexShader,
    /// GRAPHICS
    TessellationControlShader,
    /// GRAPHICS
    TesselationEvaluationShader,
    /// GRAPHICS
    GeometryShader,
    /// GRAPHICS
    EarlyFragmentTests,
    /// GRAPHICS
    FragmentShader,
    /// GRAPHICS
    LateFragmentTests,
    /// GRAPHICS
    ColorAttachmentOutput,
    /// ALL
    Bottom,
}

pub fn max_mips_2d(size: UVec2) -> u32 {
    let max_dim = size.x.max(size.y);
    32 - max_dim.leading_zeros()
}

pub fn mip_level_size_2d(texture_size: UVec2, level: u32) -> UVec2 {
    (texture_size >> level).max(UVec2::ONE)
}

#[derive(Clone, Debug)]
pub struct TextureViewDescriptor {
    pub base_mip_level: u32,
    pub mip_levels: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SurfaceFormat {
    pub format: TextureFormat,
    pub color_space: ColorSpace,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    SrgbNonLinear,
}
