use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use ash::vk;
use game_common::collections::arena::{Arena, Key};
use game_common::collections::scratch_buffer::ScratchBuffer;
use game_common::components::Color;
use game_tracing::trace_span;
use glam::UVec2;
use wgpu::hal::auxil::db;

use crate::backend::allocator::{BufferAlloc, GeneralPurposeAllocator, TextureAlloc, UsageFlags};
use crate::backend::descriptors::{AllocatedDescriptorSet, DescriptorSetAllocator};
use crate::backend::vulkan::{
    self, CommandEncoder, DescriptorSetLayout, Device, Pipeline, Sampler, TextureView,
};
use crate::backend::{
    AccessFlags, AdapterMemoryProperties, BufferBarrier, BufferUsage, CopyBuffer,
    DescriptorSetDescriptor, DescriptorType, ImageDataLayout, IndexFormat, LoadOp,
    PipelineBarriers, PipelineDescriptor, PushConstantRange, SamplerDescriptor, ShaderModule,
    ShaderSource, ShaderStages, StoreOp, TextureBarrier, TextureDescriptor, TextureFormat,
    TextureUsage, WriteDescriptorBinding, WriteDescriptorResource, WriteDescriptorResources,
};

type BufferId = Key;
type TextureId = Key;
type BindGroupId = Key;

pub struct Scheduler {
    buffers: Arena<BufferInner>,
    textures: Arena<TextureInner>,
    bind_groups: Arena<BindGroupInner>,
    allocator: GeneralPurposeAllocator,
    descriptors: DescriptorSetAllocator,
    cmds: Vec<Command>,
    device: Device,
}

impl Scheduler {
    pub fn new(device: Device, memory_props: AdapterMemoryProperties) -> Self {
        Self {
            buffers: Arena::new(),
            textures: Arena::new(),
            bind_groups: Arena::new(),
            allocator: GeneralPurposeAllocator::new(device.clone(), memory_props),
            descriptors: DescriptorSetAllocator::new(device.clone()),
            cmds: Vec::new(),
            device,
        }
    }

    pub fn queue(&mut self) -> CommandQueue<'_> {
        CommandQueue { scheduler: self }
    }

    pub fn execute(&mut self, encoder: &mut CommandEncoder<'_>) -> InflightResources<'_> {
        let _span = trace_span!("Scheduler::execute").entered();
        execute(self, encoder)
    }
}

pub struct Resources {
    pub buffers: Arena<BufferInner>,
    pub textures: Arena<TextureInner>,
    pub bind_groups: Arena<BindGroupInner>,
    pub allocator: GeneralPurposeAllocator,
    pub descriptors: DescriptorSetAllocator,
}

pub struct CommandQueue<'a> {
    scheduler: &'a mut Scheduler,
}

impl<'a> CommandQueue<'a> {
    pub fn create_buffer(&mut self, descriptor: &BufferDescriptor) -> Buffer {
        let buffer = self.scheduler.allocator.create_buffer(
            descriptor.size.try_into().unwrap(),
            BufferUsage::all(),
            UsageFlags::HOST_VISIBLE,
        );

        let id = self.scheduler.buffers.insert(BufferInner {
            buffer,
            access: AccessFlags::empty(),
            flags: BufferUsage::all(),
        });
        self.scheduler.cmds.push(Command::CreateBuffer(id));

        Buffer {
            id,
            usage: descriptor.usage,
        }
    }

    pub fn create_buffer_init(&mut self, descriptor: &BufferInitDescriptor<'_>) -> Buffer {
        let buffer = self.create_buffer(&BufferDescriptor {
            size: descriptor.contents.len() as u64,
            usage: descriptor.usage | BufferUsage::TRANSFER_DST,
        });
        self.write_buffer(&buffer, descriptor.contents);
        buffer
    }

    pub fn write_buffer(&mut self, buffer: &Buffer, data: &[u8]) {
        assert!(
            buffer.usage.contains(BufferUsage::TRANSFER_DST),
            "Buffer cannot be written to: TRANSFER_DST not set",
        );

        {
            let buffer = self.scheduler.buffers.get(buffer.id).unwrap();
            assert!(buffer.flags.contains(BufferUsage::TRANSFER_DST));
        }

        self.scheduler
            .cmds
            .push(Command::WriteBuffer(buffer.id, data.to_vec()));
    }

    #[track_caller]
    pub fn create_texture(&mut self, descriptor: &TextureDescriptor) -> Texture {
        assert!(
            !descriptor.usage.is_empty(),
            "TextureUsage flags must not be empty",
        );

        let texture = self
            .scheduler
            .allocator
            .create_texture(&descriptor, UsageFlags::HOST_VISIBLE);

        let id = self.scheduler.textures.insert(TextureInner {
            data: TextureData::Virtual(texture),
            access: AccessFlags::empty(),
        });
        self.scheduler.cmds.push(Command::CreateTexture(id));

        Texture {
            id,
            size: descriptor.size,
            format: descriptor.format,
            usage: descriptor.usage,
        }
    }

    pub fn import_texture(
        &mut self,
        texture: &'static vulkan::Texture,
        access: AccessFlags,
        size: UVec2,
        format: TextureFormat,
        usage: TextureUsage,
    ) -> Texture {
        let id = self.scheduler.textures.insert(TextureInner {
            data: TextureData::Physical(texture),
            access,
        });

        Texture {
            id,
            size,
            format,
            usage,
        }
    }

    pub(crate) fn remove_imported_texture(&mut self, texture: &Texture) {
        for (_, bind_groups) in self.scheduler.bind_groups.iter() {
            assert!(
                !bind_groups.textures.iter().any(|(_, t)| t.id == texture.id),
                "Texture cannot be removed: it is used in a descriptor set",
            );
        }

        self.scheduler.textures.remove(texture.id);
    }

    #[track_caller]
    pub fn write_texture(&mut self, texture: &Texture, data: &[u8], layout: ImageDataLayout) {
        assert!(
            texture.usage.contains(TextureUsage::TRANSFER_DST),
            "Texture cannot be written to: TRANSFER_DST usage not set",
        );

        self.scheduler
            .cmds
            .push(Command::WriteTexture(texture.id, data.to_vec(), layout));
    }

    #[track_caller]
    pub fn create_bind_group(&mut self, descriptor: &BindGroupDescriptor<'_>) -> BindGroup {
        let mut buffers = Vec::new();
        let mut samplers = Vec::new();
        let mut textures = Vec::new();
        for entry in descriptor.entries {
            match entry.resource {
                BindingResource::Buffer(buffer) => {
                    assert!(
                        buffer.usage.contains(BufferUsage::UNIFORM)
                            || buffer.usage.contains(BufferUsage::STORAGE),
                        "Buffer cannot be bound to descriptor set: UNIFORM and STORAGE not set",
                    );

                    buffers.push((entry.binding, buffer.clone()));
                }
                BindingResource::Sampler(sampler) => {
                    samplers.push((entry.binding, sampler.clone()));
                }
                BindingResource::Texture(texture) => {
                    assert!(
                        texture.usage.contains(TextureUsage::TEXTURE_BINDING),
                        "Texture cannot be bound to descriptor set: TEXTURE_BINDING not set",
                    );

                    textures.push((entry.binding, texture.clone()));
                }
            }
        }

        let id = self.scheduler.bind_groups.insert(BindGroupInner {
            buffers,
            samplers,
            textures,
            descriptor_set: None,
            layout: descriptor.layout.clone(),
            physical_texture_views: Vec::new(),
        });
        BindGroup { id }
    }

    pub fn create_descriptor_set_layout(
        &mut self,
        descriptor: &DescriptorSetDescriptor<'_>,
    ) -> DescriptorSetLayout {
        self.scheduler.device.create_descriptor_layout(descriptor)
    }

    pub fn create_pipeline(&mut self, descriptor: &PipelineDescriptor<'_>) -> Pipeline {
        self.scheduler.device.create_pipeline(descriptor)
    }

    pub fn create_sampler(&mut self, descriptor: &SamplerDescriptor) -> Sampler {
        self.scheduler.device.create_sampler(descriptor)
    }

    pub fn run_render_pass(&mut self, descriptor: &RenderPassDescriptor<'_>) -> RenderPass<'a, '_> {
        let color_attachments = descriptor
            .color_attachments
            .iter()
            .map(|a| {
                assert!(
                    a.texture.usage.contains(TextureUsage::RENDER_ATTACHMENT),
                    "Texture cannot be used as color attachment: RENDER_ATTACHMENT not set",
                );

                ColorAttachmentOwned {
                    texture: a.texture.clone(),
                    load_op: a.load_op,
                    store_op: a.store_op,
                }
            })
            .collect();

        let depth_stencil_attachment =
            descriptor
                .depth_stencil_attachment
                .map(|attachment| DepthStencilAttachmentOwned {
                    texture: attachment.texture.clone(),
                    load_op: attachment.load_op,
                    store_op: attachment.store_op,
                });

        RenderPass {
            ctx: self,
            bind_groups: HashMap::new(),
            draw_calls: Vec::new(),
            pipeline: None,
            color_attachments,
            push_constants: Vec::new(),
            index_buffer: None,
            depth_stencil_attachment,
        }
    }

    pub fn create_shader_module(&mut self, src: ShaderSource<'_>) -> ShaderModule {
        ShaderModule::new(&src, &self.scheduler.device)
    }
}

// FIXME: Remove clone bound.
#[derive(Clone, Debug)]
pub struct Buffer {
    id: BufferId,
    usage: BufferUsage,
}

#[derive(Debug)]
pub struct BufferInner {
    buffer: BufferAlloc,
    flags: BufferUsage,
    access: AccessFlags,
}

#[derive(Copy, Clone, Debug)]
pub struct BufferDescriptor {
    pub size: u64,
    pub usage: BufferUsage,
}

#[derive(Copy, Clone, Debug)]
pub struct BufferInitDescriptor<'a> {
    pub contents: &'a [u8],
    pub usage: BufferUsage,
}

pub struct BindGroupDescriptor<'a> {
    pub layout: &'a Arc<DescriptorSetLayout>,
    pub entries: &'a [BindGroupEntry<'a>],
}

pub struct BindGroupEntry<'a> {
    pub binding: u32,
    pub resource: BindingResource<'a>,
}

pub enum BindingResource<'a> {
    Buffer(&'a Buffer),
    Sampler(&'a Arc<Sampler>),
    Texture(&'a Texture),
}

#[derive(Debug)]
pub enum Command {
    CreateBuffer(BufferId),
    WriteBuffer(BufferId, Vec<u8>),
    DestroyBuffer(BufferId),
    CreateTexture(TextureId),
    WriteTexture(TextureId, Vec<u8>, ImageDataLayout),
    DestroyTexture(TextureId),
    CreateBindGroup(BindGroupId),
    RenderPass(RenderPassCmd),
}

#[derive(Clone, Debug)]
pub struct BindGroup {
    id: BindGroupId,
}

#[derive(Debug)]
pub struct BindGroupInner {
    // (Binding, Resource)
    buffers: Vec<(u32, Buffer)>,
    samplers: Vec<(u32, Arc<Sampler>)>,
    textures: Vec<(u32, Texture)>,
    descriptor_set: Option<AllocatedDescriptorSet>,
    layout: Arc<DescriptorSetLayout>,
    physical_texture_views: Vec<TextureView<'static>>,
}

#[derive(Debug)]
struct RenderPassCmd {
    pipeline: Arc<Pipeline>,
    bind_groups: HashMap<u32, BindGroup>,
    draw_calls: Vec<DrawCall>,
    color_attachments: Vec<ColorAttachmentOwned>,
    push_constants: Vec<(Vec<u8>, ShaderStages, u32)>,
    index_buffer: Option<(Buffer, IndexFormat)>,
    depth_stencil_attachment: Option<DepthStencilAttachmentOwned>,
}

#[derive(Clone, Debug)]
pub struct Texture {
    id: TextureId,
    size: UVec2,
    format: TextureFormat,
    usage: TextureUsage,
}

impl Texture {
    pub fn size(&self) -> UVec2 {
        self.size
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }
}

#[derive(Debug)]
pub struct TextureInner {
    data: TextureData,
    access: AccessFlags,
}

#[derive(Debug)]
enum TextureData {
    Physical(&'static vulkan::Texture),
    Virtual(TextureAlloc),
}

pub struct RenderPass<'a, 'b> {
    ctx: &'b mut CommandQueue<'a>,
    pipeline: Option<Arc<Pipeline>>,
    bind_groups: HashMap<u32, BindGroup>,
    draw_calls: Vec<DrawCall>,
    color_attachments: Vec<ColorAttachmentOwned>,
    push_constants: Vec<(Vec<u8>, ShaderStages, u32)>,
    index_buffer: Option<(Buffer, IndexFormat)>,
    depth_stencil_attachment: Option<DepthStencilAttachmentOwned>,
}

impl<'a, 'b> RenderPass<'a, 'b> {
    pub fn set_pipeline(&mut self, pipeline: &Arc<Pipeline>) {
        assert!(self.pipeline.is_none(), "Pipeline cannot be changed");

        self.pipeline = Some(pipeline.clone());
    }

    pub fn set_bind_group(&mut self, index: u32, bind_group: &'b BindGroup) {
        assert!(self.pipeline.is_some(), "Pipeline is not set");

        self.bind_groups.insert(index, bind_group.clone());
    }

    pub fn set_push_constants(&mut self, stages: ShaderStages, offset: u32, data: &[u8]) {
        self.push_constants.push((data.to_vec(), stages, offset));
    }

    pub fn set_index_buffer(&mut self, buffer: &Buffer, format: IndexFormat) {
        assert!(self.pipeline.is_some(), "Pipeline is not set");
        assert!(
            buffer.usage.contains(BufferUsage::INDEX),
            "Buffer cannot be used as index buffer: INDEX not set",
        );

        self.index_buffer = Some((buffer.clone(), format));
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        assert!(self.pipeline.is_some(), "Pipeline is not set");

        self.draw_calls.push(DrawCall::Draw(Draw {
            vertices,
            instances,
        }));
    }

    pub fn draw_indexed(&mut self, indices: Range<u32>, vertex_offset: i32, instances: Range<u32>) {
        self.draw_calls.push(DrawCall::DrawIndexed(DrawIndexed {
            indices,
            vertex_offset,
            instances,
        }));
    }
}

impl<'a, 'b> Drop for RenderPass<'a, 'b> {
    fn drop(&mut self) {
        if let Some(pipeline) = &self.pipeline {
            self.ctx
                .scheduler
                .cmds
                .push(Command::RenderPass(RenderPassCmd {
                    pipeline: pipeline.clone(),
                    bind_groups: self.bind_groups.clone(),
                    draw_calls: self.draw_calls.clone(),
                    color_attachments: self.color_attachments.clone(),
                    push_constants: self.push_constants.clone(),
                    index_buffer: self.index_buffer.clone(),
                    depth_stencil_attachment: self.depth_stencil_attachment.clone(),
                }));
        }
    }
}

pub struct RenderPassDescriptor<'a> {
    pub color_attachments: &'a [RenderPassColorAttachment<'a>],
    pub depth_stencil_attachment: Option<&'a DepthStencilAttachment<'a>>,
}

pub struct RenderPassColorAttachment<'a> {
    // TODO: Should be texture view
    pub texture: &'a Texture,
    pub load_op: LoadOp<Color>,
    pub store_op: StoreOp,
}

pub struct DepthStencilAttachment<'a> {
    pub texture: &'a Texture,
    pub load_op: LoadOp<f32>,
    pub store_op: StoreOp,
}

#[derive(Clone, Debug)]
struct ColorAttachmentOwned {
    texture: Texture,
    load_op: LoadOp<Color>,
    store_op: StoreOp,
}

#[derive(Clone, Debug)]
struct DepthStencilAttachmentOwned {
    texture: Texture,
    load_op: LoadOp<f32>,
    store_op: StoreOp,
}

#[derive(Clone, Debug)]
enum DrawCall {
    Draw(Draw),
    DrawIndexed(DrawIndexed),
}

#[derive(Clone, Debug)]
struct Draw {
    vertices: Range<u32>,
    instances: Range<u32>,
}

#[derive(Clone, Debug)]
struct DrawIndexed {
    indices: Range<u32>,
    vertex_offset: i32,
    instances: Range<u32>,
}

pub fn execute<'a>(
    scheduler: &'a mut Scheduler,
    encoder: &mut CommandEncoder<'_>,
) -> InflightResources<'a> {
    let mut staging_buffers = Vec::new();
    let mut frame_texture_views = Vec::new();
    let mut frame_bind_groups = Vec::new();

    for cmd in scheduler.cmds.drain(..) {
        match cmd {
            Command::CreateBuffer(buffer) => {
                // Nothing to do
            }
            Command::WriteBuffer(id, data) => {
                let buffer = scheduler.buffers.get_mut(id).unwrap();

                unsafe {
                    buffer.buffer.map().copy_from_slice(&data);
                }
            }
            Command::CreateTexture(id) => {
                // Nothing to do
            }
            Command::WriteTexture(id, data, layout) => {
                let texture = scheduler.textures.get_mut(id).unwrap();

                let mut staging_buffer = scheduler.allocator.create_buffer(
                    (data.len() as u64).try_into().unwrap(),
                    BufferUsage::TRANSFER_SRC,
                    UsageFlags::HOST_VISIBLE,
                );

                unsafe {
                    staging_buffer.map().copy_from_slice(&data);
                }

                match &mut texture.data {
                    TextureData::Physical(_) => todo!(),
                    TextureData::Virtual(tex) => {
                        encoder.insert_pipeline_barriers(&PipelineBarriers {
                            buffer: &[],
                            texture: &[TextureBarrier {
                                src_access: texture.access,
                                dst_access: AccessFlags::TRANSFER_WRITE,
                                texture: tex.texture(),
                            }],
                        });

                        encoder.copy_buffer_to_texture(
                            CopyBuffer {
                                buffer: staging_buffer.buffer(),
                                offset: 0,
                                layout,
                            },
                            tex.texture(),
                        );

                        texture.access = AccessFlags::TRANSFER_WRITE;
                        staging_buffers.push(staging_buffer);
                    }
                }
            }
            Command::CreateBindGroup(id) => {
                // Nothing to do
            }
            Command::RenderPass(cmd) => {
                let mut sets = Vec::new();

                let mut buffer_accesses = HashMap::<BufferId, AccessFlags>::new();
                let mut texture_accesses = HashMap::<TextureId, AccessFlags>::new();

                for (index, bind_group_e) in &cmd.bind_groups {
                    let bind_group = scheduler.bind_groups.get_mut(bind_group_e.id).unwrap();

                    bind_group.descriptor_set.get_or_insert_with(|| {
                        let mut physical_bindings = Vec::new();

                        let buffer_views = ScratchBuffer::new(bind_group.buffers.len());
                        let texture_views = ScratchBuffer::new(bind_group.textures.len());

                        for (binding, buffer) in &bind_group.buffers {
                            *buffer_accesses.entry(buffer.id).or_default() |=
                                AccessFlags::SHADER_READ;

                            let buffer = scheduler.buffers.get(buffer.id).unwrap();

                            let view = buffer_views.insert(buffer.buffer.buffer_view());

                            match bind_group.layout.bindings()[*binding as usize].kind {
                                DescriptorType::Uniform => {
                                    physical_bindings.push(WriteDescriptorBinding {
                                        binding: *binding,
                                        resource: WriteDescriptorResource::UniformBuffer(view),
                                    });
                                }
                                DescriptorType::Storage => {
                                    physical_bindings.push(WriteDescriptorBinding {
                                        binding: *binding,
                                        resource: WriteDescriptorResource::StorageBuffer(view),
                                    });
                                }
                                _ => unreachable!(),
                            }
                        }

                        for (binding, texture) in &bind_group.textures {
                            *texture_accesses.entry(texture.id).or_default() |=
                                AccessFlags::SHADER_READ;

                            let texture = scheduler.textures.get(texture.id).unwrap();

                            let physical_texture = match &texture.data {
                                TextureData::Physical(data) => data,
                                TextureData::Virtual(data) => data.texture(),
                            };

                            let view = texture_views.insert(physical_texture.create_view());

                            physical_bindings.push(WriteDescriptorBinding {
                                binding: *binding,
                                resource: WriteDescriptorResource::Texture(view),
                            });
                        }

                        for (binding, sampler) in &bind_group.samplers {
                            physical_bindings.push(WriteDescriptorBinding {
                                binding: *binding,
                                resource: WriteDescriptorResource::Sampler(sampler),
                            });
                        }

                        physical_bindings.sort_by(|a, b| a.binding.cmp(&b.binding));

                        let mut set =
                            unsafe { scheduler.descriptors.alloc(&bind_group.layout).unwrap() };
                        set.raw_mut().update(&WriteDescriptorResources {
                            bindings: &physical_bindings,
                        });

                        bind_group.physical_texture_views.extend(texture_views);

                        set
                    });

                    sets.push((*index, bind_group_e.id));
                }

                let attachment_views = ScratchBuffer::new(
                    cmd.color_attachments.len()
                        + usize::from(cmd.depth_stencil_attachment.is_some()),
                );
                let mut color_attachments = Vec::new();
                for attachment in cmd.color_attachments {
                    let texture = scheduler.textures.get(attachment.texture.id).unwrap();
                    let physical_texture = match &texture.data {
                        TextureData::Physical(data) => data,
                        TextureData::Virtual(data) => data.texture(),
                    };

                    *texture_accesses.entry(attachment.texture.id).or_default() |=
                        AccessFlags::COLOR_ATTACHMENT_WRITE;

                    let view = attachment_views.insert(physical_texture.create_view());

                    color_attachments.push(crate::backend::RenderPassColorAttachment {
                        load_op: attachment.load_op,
                        store_op: attachment.store_op,
                        view,
                        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    });
                }

                let depth_attachment = cmd.depth_stencil_attachment.map(|attachment| {
                    let texture = scheduler.textures.get(attachment.texture.id).unwrap();
                    let physical_texture = match &texture.data {
                        TextureData::Physical(data) => data,
                        TextureData::Virtual(data) => data.texture(),
                    };

                    *texture_accesses.entry(attachment.texture.id).or_default() |=
                        AccessFlags::DEPTH_ATTACHMENT_READ | AccessFlags::DEPTH_ATTACHMENT_WRITE;

                    let view = attachment_views.insert(physical_texture.create_view());

                    crate::backend::RenderPassDepthStencilAttachment {
                        depth_load_op: attachment.load_op,
                        depth_store_op: attachment.store_op,
                        view,
                        layout: vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
                    }
                });

                if let Some((buffer, _)) = &cmd.index_buffer {
                    *buffer_accesses.entry(buffer.id).or_default() |= AccessFlags::INDEX;
                }

                let mut buffer_barriers = Vec::new();
                for (buffer_id, dst_access) in &buffer_accesses {
                    let buffer = scheduler.buffers.get(*buffer_id).unwrap();

                    buffer_barriers.push(BufferBarrier {
                        buffer: buffer.buffer.buffer(),
                        offset: 0,
                        size: buffer.buffer.size(),
                        src_access: buffer.access,
                        dst_access: *dst_access,
                    });
                }

                let mut texture_barriers = Vec::new();
                for (texture_id, dst_access) in &texture_accesses {
                    let texture = scheduler.textures.get(*texture_id).unwrap();

                    let texture_data = match &texture.data {
                        TextureData::Physical(data) => data,
                        TextureData::Virtual(data) => data.texture(),
                    };

                    texture_barriers.push(TextureBarrier {
                        texture: texture_data,
                        src_access: texture.access,
                        dst_access: *dst_access,
                    });
                }

                encoder.insert_pipeline_barriers(&PipelineBarriers {
                    buffer: &buffer_barriers,
                    texture: &texture_barriers,
                });

                let mut render_pass =
                    encoder.begin_render_pass(&crate::backend::RenderPassDescriptor {
                        color_attachments: &color_attachments,
                        depth_stencil_attachment: depth_attachment.as_ref(),
                    });

                render_pass.bind_pipeline(&cmd.pipeline);
                for (index, set) in sets {
                    let set = scheduler
                        .bind_groups
                        .get(set)
                        .unwrap()
                        .descriptor_set
                        .as_ref()
                        .unwrap();
                    render_pass.bind_descriptor_set(index, set.raw());
                }

                for (data, stages, offset) in cmd.push_constants {
                    render_pass.set_push_constants(stages, offset, &data);
                }

                if let Some((buffer, format)) = cmd.index_buffer {
                    let buffer = scheduler.buffers.get(buffer.id).unwrap();
                    render_pass.bind_index_buffer(buffer.buffer.buffer_view(), format);
                }

                for call in cmd.draw_calls {
                    match call {
                        DrawCall::Draw(call) => {
                            render_pass.draw(call.vertices, call.instances);
                        }
                        DrawCall::DrawIndexed(call) => {
                            render_pass.draw_indexed(
                                call.indices,
                                call.vertex_offset,
                                call.instances,
                            );
                        }
                    }
                }

                drop(render_pass);
                frame_texture_views.push(attachment_views);

                for (buffer_id, dst_access) in buffer_accesses {
                    let buffer = scheduler.buffers.get_mut(buffer_id).unwrap();
                    buffer.access = dst_access;
                }

                for (texture_id, dst_access) in texture_accesses {
                    let texture = scheduler.textures.get_mut(texture_id).unwrap();
                    texture.access = dst_access;
                }
            }
            _ => todo!(),
        }
    }

    InflightResources {
        texture_views: frame_texture_views,
        staging_buffers,
        frame_bind_groups,
    }
}

#[derive(Debug)]
pub struct InflightResources<'a> {
    texture_views: Vec<ScratchBuffer<TextureView<'a>>>,
    staging_buffers: Vec<BufferAlloc>,
    frame_bind_groups: Vec<BindGroupId>,
}
