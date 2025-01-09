use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use ash::vk;
use game_common::collections::arena::{Arena, Key};
use game_common::collections::scratch_buffer::ScratchBuffer;

use crate::backend::allocator::{BufferAlloc, GeneralPurposeAllocator, TextureAlloc, UsageFlags};
use crate::backend::descriptors::{AllocatedDescriptorSet, DescriptorSetAllocator};
use crate::backend::vulkan::{
    self, CommandEncoder, DescriptorSet, DescriptorSetLayout, Device, Pipeline, Sampler,
    ShaderModule, TextureView,
};
use crate::backend::{
    AccessFlags, BufferBarrier, BufferUsage, CopyBuffer, DescriptorBinding,
    DescriptorSetDescriptor, ImageDataLayout, LoadOp, PipelineBarriers, PipelineDescriptor,
    SamplerDescriptor, StoreOp, TextureBarrier, TextureDescriptor, TextureLayout,
    WriteDescriptorBinding, WriteDescriptorResource, WriteDescriptorResources,
};

type BufferId = Key;
type TextureId = Key;
type BindGroupId = Key;

pub struct Resources {
    pub buffers: Arena<BufferInner>,
    pub textures: Arena<TextureInner>,
    pub bind_groups: Arena<BindGroupInner>,
    pub allocator: GeneralPurposeAllocator,
    pub descriptors: DescriptorSetAllocator,
}

pub struct RenderContext<'a> {
    pub device: &'a Device,
    pub resources: &'a mut Resources,
    pub cmds: Vec<Command>,
}

impl<'a> RenderContext<'a> {
    pub fn create_buffer(&mut self, descriptor: BufferDescriptor) -> Buffer {
        let buffer = self.resources.allocator.create_buffer(
            descriptor.size.try_into().unwrap(),
            BufferUsage::all(),
            UsageFlags::HOST_VISIBLE,
        );

        let id = self.resources.buffers.insert(BufferInner {
            buffer,
            access: AccessFlags::empty(),
            flags: BufferUsage::all(),
        });
        self.cmds.push(Command::CreateBuffer(id));

        Buffer { id }
    }

    pub fn write_buffer(&mut self, buffer: &Buffer, data: &[u8]) {
        {
            let buffer = self.resources.buffers.get(buffer.id).unwrap();
            assert!(buffer.flags.contains(BufferUsage::TRANSFER_DST));
        }

        self.cmds
            .push(Command::WriteBuffer(buffer.id, data.to_vec()));
    }

    pub fn create_texture(&mut self, descriptor: TextureDescriptor) -> Texture {
        let texture = self
            .resources
            .allocator
            .create_texture(&descriptor, UsageFlags::HOST_VISIBLE);

        let id = self.resources.textures.insert(TextureInner {
            data: TextureData::Virtual(texture),
            access: AccessFlags::empty(),
        });
        self.cmds.push(Command::CreateTexture(id));

        Texture { id }
    }

    pub fn import_texture(
        &mut self,
        texture: &'static vulkan::Texture,
        access: AccessFlags,
    ) -> Texture {
        let id = self.resources.textures.insert(TextureInner {
            data: TextureData::Physical(texture),
            access,
        });
        Texture { id }
    }

    pub fn write_texture(&mut self, texture: &Texture, data: &[u8], layout: ImageDataLayout) {
        self.cmds
            .push(Command::WriteTexture(texture.id, data.to_vec(), layout));
    }

    pub fn create_bind_group(&mut self, descriptor: BindGroupDescriptor<'_>) -> BindGroup {
        let mut buffers = Vec::new();
        let mut samplers = Vec::new();
        let mut textures = Vec::new();
        for entry in descriptor.entries {
            match entry.resource {
                BindingResource::Buffer(buffer) => {
                    buffers.push((entry.binding, buffer.clone()));
                }
                BindingResource::Sampler(sampler) => {
                    samplers.push((entry.binding, sampler.clone()));
                }
                BindingResource::Texture(texture) => {
                    textures.push((entry.binding, texture.clone()));
                }
            }
        }

        let id = self.resources.bind_groups.insert(BindGroupInner {
            buffers,
            samplers,
            textures,
            descriptor_set: None,
            layout: descriptor.layout.clone(),
        });
        BindGroup { id }
    }

    pub fn create_shader(&mut self, code: &[u32]) -> ShaderModule {
        unsafe { self.device.create_shader(code) }
    }

    pub fn create_descriptor_set_layout(
        &mut self,
        descriptor: &DescriptorSetDescriptor<'_>,
    ) -> DescriptorSetLayout {
        self.device.create_descriptor_layout(descriptor)
    }

    pub fn create_pipeline(&mut self, descriptor: &PipelineDescriptor<'_>) -> Pipeline {
        self.device.create_pipeline(descriptor)
    }

    pub fn create_sampler(&mut self, descriptor: &SamplerDescriptor) -> Sampler {
        self.device.create_sampler(descriptor)
    }

    pub fn run_render_pass(&mut self, descriptor: &RenderPassDescriptor<'_>) -> RenderPass<'a, '_> {
        let color_attachments = descriptor
            .color_attachments
            .iter()
            .map(|a| ColorAttachmentOwned {
                texture: a.texture.clone(),
                load_op: a.load_op,
                store_op: a.store_op,
            })
            .collect();

        RenderPass {
            ctx: self,
            bind_groups: HashMap::new(),
            draw_calls: Vec::new(),
            pipeline: None,
            color_attachments,
        }
    }
}

// FIXME: Remove clone bound.
#[derive(Clone, Debug)]
pub struct Buffer {
    id: BufferId,
}

pub struct BufferInner {
    buffer: BufferAlloc,
    flags: BufferUsage,
    access: AccessFlags,
}

#[derive(Copy, Clone, Debug)]
pub struct BufferDescriptor {
    pub size: u64,
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

pub struct BindGroupInner {
    // (Binding, Resource)
    buffers: Vec<(u32, Buffer)>,
    samplers: Vec<(u32, Arc<Sampler>)>,
    textures: Vec<(u32, Texture)>,
    descriptor_set: Option<AllocatedDescriptorSet>,
    layout: Arc<DescriptorSetLayout>,
}

struct RenderPassCmd {
    pipeline: Arc<Pipeline>,
    bind_groups: HashMap<u32, BindGroup>,
    draw_calls: Vec<DrawCall>,
    color_attachments: Vec<ColorAttachmentOwned>,
}

#[derive(Clone, Debug)]
pub struct Texture {
    id: TextureId,
}

pub struct TextureInner {
    data: TextureData,
    access: AccessFlags,
}

enum TextureData {
    Physical(&'static vulkan::Texture),
    Virtual(TextureAlloc),
}

pub struct RenderPass<'a, 'b> {
    ctx: &'b mut RenderContext<'a>,
    pipeline: Option<Arc<Pipeline>>,
    bind_groups: HashMap<u32, BindGroup>,
    draw_calls: Vec<DrawCall>,
    color_attachments: Vec<ColorAttachmentOwned>,
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

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        assert!(self.pipeline.is_some(), "Pipeline is not set");

        self.draw_calls.push(DrawCall {
            vertices,
            instances,
        });
    }
}

impl<'a, 'b> Drop for RenderPass<'a, 'b> {
    fn drop(&mut self) {
        if let Some(pipeline) = &self.pipeline {
            self.ctx.cmds.push(Command::RenderPass(RenderPassCmd {
                pipeline: pipeline.clone(),
                bind_groups: self.bind_groups.clone(),
                draw_calls: self.draw_calls.clone(),
                color_attachments: self.color_attachments.clone(),
            }));
        }
    }
}

pub struct RenderPassDescriptor<'a> {
    pub color_attachments: &'a [RenderPassColorAttachment<'a>],
}

pub struct RenderPassColorAttachment<'a> {
    // TODO: Should be texture view
    pub texture: &'a Texture,
    pub load_op: LoadOp,
    pub store_op: StoreOp,
}

#[derive(Clone, Debug)]
struct ColorAttachmentOwned {
    texture: Texture,
    load_op: LoadOp,
    store_op: StoreOp,
}

#[derive(Clone, Debug)]
struct DrawCall {
    vertices: Range<u32>,
    instances: Range<u32>,
}

pub fn execute<'a, I>(
    resources: &'a mut Resources,
    cmds: I,
    encoder: &mut CommandEncoder<'_>,
) -> InflightResources<'a>
where
    I: IntoIterator<Item = Command>,
{
    let mut staging_buffers = Vec::new();
    let mut frame_texture_views = Vec::new();

    for cmd in cmds.into_iter() {
        match cmd {
            Command::CreateBuffer(buffer) => {
                // Nothing to do
            }
            Command::WriteBuffer(id, data) => {
                let buffer = resources.buffers.get_mut(id).unwrap();

                unsafe {
                    buffer.buffer.map().copy_from_slice(&data);
                }
            }
            Command::CreateTexture(id) => {
                // Nothing to do
            }
            Command::WriteTexture(id, data, layout) => {
                let texture = resources.textures.get_mut(id).unwrap();

                let mut staging_buffer = resources.allocator.create_buffer(
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
                let mut buffer_barriers = Vec::new();
                let mut texture_barriers = Vec::new();
                let mut buffer_transition = Vec::new();
                let mut texture_transition = Vec::new();

                let mut sets = Vec::new();

                for (index, bind_group_e) in &cmd.bind_groups {
                    let bind_group = resources.bind_groups.get_mut(bind_group_e.id).unwrap();

                    bind_group.descriptor_set.get_or_insert_with(|| {
                        let mut physical_bindings = Vec::new();

                        let buffer_views = ScratchBuffer::new(bind_group.buffers.len());
                        let texture_views = ScratchBuffer::new(bind_group.textures.len());

                        for (binding, buffer) in &bind_group.buffers {
                            buffer_transition.push((buffer.id, AccessFlags::SHADER_READ));
                            let buffer = resources.buffers.get(buffer.id).unwrap();

                            buffer_barriers.push(BufferBarrier {
                                buffer: buffer.buffer.buffer(),
                                src_access: AccessFlags::empty(),
                                dst_access: AccessFlags::SHADER_READ,
                                offset: 0,
                                size: buffer.buffer.size(),
                            });

                            let view = buffer_views.insert(buffer.buffer.buffer_view());

                            physical_bindings.push(WriteDescriptorBinding {
                                binding: *binding,
                                resource: WriteDescriptorResource::Buffer(view),
                            });
                        }

                        for (binding, texture) in &bind_group.textures {
                            texture_transition.push((texture.id, AccessFlags::SHADER_READ));
                            let texture = resources.textures.get(texture.id).unwrap();

                            let physical_texture = match &texture.data {
                                TextureData::Physical(data) => data,
                                TextureData::Virtual(data) => data.texture(),
                            };

                            texture_barriers.push(TextureBarrier {
                                texture: physical_texture,
                                src_access: texture.access,
                                dst_access: AccessFlags::SHADER_READ,
                            });

                            let view = physical_texture.create_view();
                            let view = texture_views.insert(view);

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

                        let mut set = unsafe { resources.descriptors.alloc(&bind_group.layout) };
                        set.raw_mut().update(&WriteDescriptorResources {
                            bindings: &physical_bindings,
                        });

                        frame_texture_views.push(texture_views);

                        set
                    });

                    sets.push((*index, bind_group_e.id));
                }

                let color_attachment_views = ScratchBuffer::new(cmd.color_attachments.len());
                let mut color_attachments = Vec::new();
                for attachment in cmd.color_attachments {
                    let texture = resources.textures.get(attachment.texture.id).unwrap();
                    let texture = match &texture.data {
                        TextureData::Physical(data) => data,
                        TextureData::Virtual(data) => data.texture(),
                    };

                    let view = color_attachment_views.insert(texture.create_view());

                    color_attachments.push(crate::backend::RenderPassColorAttachment {
                        load_op: attachment.load_op,
                        store_op: attachment.store_op,
                        view,
                        size: texture.size(),
                        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    });
                }

                encoder.insert_pipeline_barriers(&PipelineBarriers {
                    buffer: &buffer_barriers,
                    texture: &texture_barriers,
                });

                let mut render_pass =
                    encoder.begin_render_pass(&crate::backend::RenderPassDescriptor {
                        color_attachments: &color_attachments,
                    });

                render_pass.bind_pipeline(&cmd.pipeline);
                for (index, set) in sets {
                    let set = resources
                        .bind_groups
                        .get(set)
                        .unwrap()
                        .descriptor_set
                        .as_ref()
                        .unwrap();
                    render_pass.bind_descriptor_set(index, set.raw());
                }

                for call in cmd.draw_calls {
                    render_pass.draw(call.vertices, call.instances);
                }

                drop(render_pass);
                frame_texture_views.push(color_attachment_views);

                for (id, access) in buffer_transition {
                    resources.buffers.get_mut(id).unwrap().access = access;
                }

                for (id, access) in texture_transition {
                    resources.textures.get_mut(id).unwrap().access = access;
                }
            }
            _ => todo!(),
        }
    }

    InflightResources {
        texture_views: frame_texture_views,
        staging_buffers,
    }
}

pub struct InflightResources<'a> {
    staging_buffers: Vec<BufferAlloc>,
    texture_views: Vec<ScratchBuffer<TextureView<'a>>>,
}
