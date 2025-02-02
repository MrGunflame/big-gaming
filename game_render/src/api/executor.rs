use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use ash::ext::pci_bus_info;
use ash::khr::acceleration_structure;
use ash::vk;
use game_common::collections::scratch_buffer::ScratchBuffer;
use game_tracing::trace_span;

use crate::backend::allocator::{BufferAlloc, UsageFlags};
use crate::backend::vulkan::{CommandEncoder, TextureView};
use crate::backend::{
    BufferBarrier, CopyBuffer, DescriptorType, ImageDataLayout, PipelineBarriers,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    TextureBarrier, TextureViewDescriptor, WriteDescriptorBinding, WriteDescriptorResource,
    WriteDescriptorResources,
};

use super::scheduler::{Barrier, Step};
use super::{
    Buffer, BufferId, Command, CopyBufferToBuffer, CopyBufferToTexture, DescriptorSet,
    DescriptorSetId, Draw, DrawCall, DrawCmd, LifecycleEvent, PipelineId, RenderPassCmd,
    ResourceId, Resources, SamplerId, Texture, TextureData, TextureId,
};

pub fn execute<'a, I>(
    resources: &mut Resources,
    steps: I,
    encoder: &mut CommandEncoder<'_>,
) -> TemporaryResources
where
    I: IntoIterator<Item = Step<&'a Command, ResourceId>>,
{
    let _span = trace_span!("execute").entered();

    let mut tmp = TemporaryResources::default();

    let mut barriers = Vec::new();

    for step in steps {
        // Batch all barrier steps together, then when the new step is not
        // a barrier emit all barriers at once.
        if !barriers.is_empty() && !step.is_barrier() {
            insert_barriers(resources, &barriers, encoder);
            barriers.clear();
        }

        match step {
            Step::Node(Command::WriteBuffer(id, data)) => {
                write_buffer(resources, &mut tmp, *id, data);
            }
            Step::Node(Command::CopyBufferToBuffer(cmd)) => {
                copy_buffer_to_buffer(resources, &mut tmp, cmd, encoder);
            }
            Step::Node(Command::CopyBufferToTexture(cmd)) => {
                copy_buffer_to_texture(resources, &mut tmp, cmd, encoder);
            }
            Step::Node(Command::RenderPass(cmd)) => {
                run_render_pass(resources, &mut tmp, &cmd, encoder);
            }
            Step::Node(Command::TextureTransition(texture, _)) => {
                // The texture is not explicitly used anywhere else, but
                // it still must be kept alive for this frame since it
                // is used in a barrier.
                tmp.textures.insert(texture.id);
            }
            Step::Barrier(barrier) => {
                barriers.push(barrier);
            }
        }
    }

    // Flush any remaining barriers.
    if !barriers.is_empty() {
        insert_barriers(resources, &barriers, encoder);
    }

    tmp
}

fn write_buffer(
    resources: &mut Resources,
    tmp: &mut TemporaryResources,
    id: BufferId,
    data: &[u8],
) {
    let buffer = resources.buffers.get_mut(id).unwrap();

    unsafe {
        buffer.buffer.map().copy_from_slice(&data);
    }

    tmp.buffers.insert(id);
}

fn copy_buffer_to_buffer(
    resources: &mut Resources,
    tmp: &mut TemporaryResources,
    cmd: &CopyBufferToBuffer,
    encoder: &mut CommandEncoder<'_>,
) {
    let src = resources.buffers.get(cmd.src).unwrap();
    let dst = resources.buffers.get(cmd.dst).unwrap();

    encoder.copy_buffer_to_buffer(
        src.buffer.buffer(),
        cmd.src_offset,
        dst.buffer.buffer(),
        cmd.dst_offset,
        cmd.count.get(),
    );

    // Both buffers must be kept alive until the copy
    // operation is complete.
    tmp.buffers.insert(cmd.src);
    tmp.buffers.insert(cmd.dst);
}

fn copy_buffer_to_texture(
    resources: &mut Resources,
    tmp: &mut TemporaryResources,
    cmd: &CopyBufferToTexture,
    encoder: &mut CommandEncoder<'_>,
) {
    let buffer = resources.buffers.get(cmd.src).unwrap();
    let texture = resources.textures.get(cmd.dst).unwrap();

    encoder.copy_buffer_to_texture(
        CopyBuffer {
            buffer: buffer.buffer.buffer(),
            offset: cmd.src_offset,
            layout: cmd.layout,
        },
        texture.data.texture(),
        cmd.dst_mip_level,
    );

    // Both buffer and texture must be kept alive until the
    // copy operation is complete.
    tmp.buffers.insert(cmd.src);
    tmp.textures.insert(cmd.dst);
}

fn run_render_pass(
    resources: &mut Resources,
    tmp: &mut TemporaryResources,
    cmd: &RenderPassCmd,
    encoder: &mut CommandEncoder<'_>,
) {
    // Ensure all physical descriptor sets are created before
    // the render pass begins.
    for cmd in &cmd.cmds {
        match cmd {
            DrawCmd::SetDescriptorSet(_, id) => {
                let set = resources.descriptor_sets.get_mut(*id).unwrap();
                if set.descriptor_set.is_none() {
                    build_descriptor_set(resources, *id);
                }
            }
            _ => (),
        }
    }

    let attachment_views = ScratchBuffer::new(
        cmd.color_attachments.len() + usize::from(cmd.depth_stencil_attachment.is_some()),
    );

    let mut color_attachments = Vec::new();
    for attachment in &cmd.color_attachments {
        let texture = resources.textures.get(attachment.target.texture).unwrap();
        let physical_texture = match &texture.data {
            TextureData::Physical(data) => data,
            TextureData::Virtual(data) => data.texture(),
        };

        // Attachment must be kept alive until the render pass completes.
        tmp.textures.insert(attachment.target.texture);

        let view = attachment_views.insert(unsafe {
            physical_texture
                .create_view(&TextureViewDescriptor {
                    base_mip_level: attachment.target.base_mip_level,
                    mip_levels: attachment.target.mip_levels,
                })
                .make_static()
        });

        color_attachments.push(RenderPassColorAttachment {
            view,
            load_op: attachment.load_op,
            store_op: attachment.store_op,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        });
    }

    let depth_attachment = cmd.depth_stencil_attachment.as_ref().map(|attachment| {
        let texture = resources.textures.get(attachment.texture).unwrap();
        let physical_texture = match &texture.data {
            TextureData::Physical(data) => data,
            TextureData::Virtual(data) => data.texture(),
        };

        // Attachment must be kept alive until the render pass completes.
        tmp.textures.insert(attachment.texture);

        // Depth stencil attachment can only be a single mip.
        let view = attachment_views.insert(unsafe {
            physical_texture
                .create_view(&TextureViewDescriptor {
                    base_mip_level: 0,
                    mip_levels: 1,
                })
                .make_static()
        });

        RenderPassDepthStencilAttachment {
            depth_load_op: attachment.load_op,
            depth_store_op: attachment.store_op,
            view,
            layout: vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
        }
    });

    let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
        color_attachments: &color_attachments,
        depth_stencil_attachment: depth_attachment.as_ref(),
    });

    for cmd in &cmd.cmds {
        match cmd {
            DrawCmd::SetPipeline(id) => {
                let pipeline = resources.pipelines.get(*id).unwrap();
                render_pass.bind_pipeline(&pipeline.inner);

                // Pipeline must be kept alive until the render pass
                // completes.
                tmp.pipelines.insert(*id);
            }
            DrawCmd::SetDescriptorSet(index, id) => {
                let set = resources
                    .descriptor_sets
                    .get(*id)
                    .unwrap()
                    .descriptor_set
                    .as_ref()
                    .unwrap()
                    .raw();
                render_pass.bind_descriptor_set(*index, set);

                // Descriptor set must be kept alive until the render pass
                // completes.
                tmp.descriptor_sets.insert(*id);
            }
            DrawCmd::SetIndexBuffer(id, format) => {
                let buffer = resources.buffers.get(*id).unwrap();
                render_pass.bind_index_buffer(buffer.buffer.buffer_view(), *format);

                // Index buffer must be kept alive until the render pass
                // completes.
                tmp.buffers.insert(*id);
            }
            DrawCmd::SetPushConstants(data, stages, offset) => {
                render_pass.set_push_constants(*stages, *offset, data);
            }
            DrawCmd::Draw(DrawCall::Draw(call)) => {
                render_pass.draw(call.vertices.clone(), call.instances.clone());
            }
            DrawCmd::Draw(DrawCall::DrawIndexed(call)) => {
                render_pass.draw_indexed(
                    call.indices.clone(),
                    call.vertex_offset,
                    call.instances.clone(),
                );
            }
        }
    }

    drop(render_pass);

    tmp.texture_views.extend(attachment_views);
}

fn build_descriptor_set(resources: &mut Resources, id: DescriptorSetId) {
    let _span = trace_span!("build_descriptor_set").entered();

    let descriptor_set = resources.descriptor_sets.get_mut(id).unwrap();
    let layout = resources
        .descriptor_set_layouts
        .get(descriptor_set.layout)
        .unwrap();

    let mut bindings = Vec::new();

    let buffer_views = ScratchBuffer::new(descriptor_set.buffers.len());
    let texture_views = ScratchBuffer::new(descriptor_set.textures.len());
    let texture_array_views = ScratchBuffer::new(descriptor_set.texture_arrays.len());

    for (binding, id) in &descriptor_set.buffers {
        let buffer = resources.buffers.get(*id).unwrap();

        let view = buffer_views.insert(buffer.buffer.buffer_view());

        match layout.inner.bindings()[*binding as usize].kind {
            DescriptorType::Uniform => {
                bindings.push(WriteDescriptorBinding {
                    binding: *binding,
                    resource: WriteDescriptorResource::UniformBuffer(core::slice::from_ref(view)),
                });
            }
            DescriptorType::Storage => {
                bindings.push(WriteDescriptorBinding {
                    binding: *binding,
                    resource: WriteDescriptorResource::StorageBuffer(core::slice::from_ref(view)),
                });
            }
            _ => unreachable!(),
        }
    }

    for (binding, view) in &descriptor_set.textures {
        let texture = resources.textures.get(view.texture).unwrap();

        let physical_texture = match &texture.data {
            TextureData::Physical(data) => data,
            TextureData::Virtual(data) => data.texture(),
        };

        let view = texture_views.insert(unsafe {
            physical_texture
                .create_view(&TextureViewDescriptor {
                    base_mip_level: view.base_mip_level,
                    mip_levels: view.mip_levels,
                })
                .make_static()
        });

        bindings.push(WriteDescriptorBinding {
            binding: *binding,
            resource: WriteDescriptorResource::Texture(core::slice::from_ref(view)),
        });
    }

    for (binding, id) in &descriptor_set.samplers {
        let sampler = resources.samplers.get(*id).unwrap();
        bindings.push(WriteDescriptorBinding {
            binding: *binding,
            resource: WriteDescriptorResource::Sampler(core::slice::from_ref(&sampler.inner)),
        });
    }

    for (binding, textures) in &descriptor_set.texture_arrays {
        let views = texture_array_views.insert(ScratchBuffer::new(textures.len()));

        for view in textures {
            let texture = resources.textures.get(view.texture).unwrap();
            let physical_texture = match &texture.data {
                TextureData::Physical(data) => data,
                TextureData::Virtual(data) => data.texture(),
            };

            views.insert(unsafe {
                physical_texture
                    .create_view(&TextureViewDescriptor {
                        base_mip_level: view.base_mip_level,
                        mip_levels: view.mip_levels,
                    })
                    .make_static()
            });
        }

        bindings.push(WriteDescriptorBinding {
            binding: *binding,
            resource: WriteDescriptorResource::Texture(views.as_slice()),
        });
    }

    let mut set = unsafe { resources.descriptor_allocator.alloc(&layout.inner).unwrap() };
    set.raw_mut().update(&WriteDescriptorResources {
        bindings: &bindings,
    });

    descriptor_set.physical_texture_views.extend(texture_views);
    for texture_views in texture_array_views {
        descriptor_set.physical_texture_views.extend(texture_views);
    }

    descriptor_set.descriptor_set = Some(set);
}

fn insert_barriers(
    resources: &Resources,
    barriers: &[Barrier<ResourceId>],
    encoder: &mut CommandEncoder<'_>,
) {
    let mut buffer_barriers = Vec::new();
    let mut texture_barriers = Vec::new();

    for barrier in barriers {
        match barrier.resource {
            ResourceId::Buffer(id) => {
                let buffer = resources.buffers.get(id).unwrap();
                buffer_barriers.push(BufferBarrier {
                    buffer: buffer.buffer.buffer(),
                    offset: buffer.buffer.buffer_view().offset(),
                    size: buffer.buffer.buffer_view().len(),
                    src_access: barrier.src_access,
                    dst_access: barrier.dst_access,
                });
            }
            ResourceId::Texture(tex) => {
                let texture = resources.textures.get(tex.id).unwrap();
                texture_barriers.push(TextureBarrier {
                    texture: texture.data.texture(),
                    src_access: barrier.src_access,
                    dst_access: barrier.dst_access,
                    base_mip_level: tex.mip_level,
                    mip_levels: 1,
                });
            }
        }
    }

    debug_assert!(!buffer_barriers.is_empty() || !texture_barriers.is_empty());
    encoder.insert_pipeline_barriers(&PipelineBarriers {
        buffer: &buffer_barriers,
        texture: &texture_barriers,
    });
}

#[derive(Debug, Default)]
pub struct TemporaryResources {
    staging_buffers: Vec<BufferAlloc>,
    texture_views: Vec<TextureView<'static>>,
    descriptor_sets: CountingSet<DescriptorSetId>,
    buffers: CountingSet<BufferId>,
    textures: CountingSet<TextureId>,
    pipelines: CountingSet<PipelineId>,
    samplers: CountingSet<SamplerId>,
}

impl TemporaryResources {
    pub(super) fn destroy(mut self, resources: &mut Resources) {
        drop(self.texture_views.drain(..));

        for (id, count) in self.buffers.drain() {
            for _ in 0..count {
                resources
                    .lifecycle_events_tx
                    .send(LifecycleEvent::DestroyBufferHandle(id));
            }
        }

        for (id, count) in self.textures.drain() {
            for _ in 0..count {
                resources
                    .lifecycle_events_tx
                    .send(LifecycleEvent::DestroyTextureHandle(id));
            }
        }

        for (id, count) in self.descriptor_sets.drain() {
            for _ in 0..count {
                resources
                    .lifecycle_events_tx
                    .send(LifecycleEvent::DestroyDescriptorSetHandle(id));
            }
        }

        for (id, count) in self.samplers.drain() {
            for _ in 0..count {
                resources
                    .lifecycle_events_tx
                    .send(LifecycleEvent::DestroySamplerHandle(id));
            }
        }

        for (id, count) in self.pipelines.drain() {
            for _ in 0..count {
                resources
                    .lifecycle_events_tx
                    .send(LifecycleEvent::DestroyPipelineHandle(id));
            }
        }
    }
}

#[derive(Clone, Debug)]
struct CountingSet<T>(HashMap<T, usize>);

impl<T> CountingSet<T> {
    fn new() -> Self {
        Self(HashMap::new())
    }
}

impl<T> CountingSet<T>
where
    T: Hash + Eq,
{
    fn insert(&mut self, key: T) {
        *self.0.entry(key).or_default() += 1;
    }

    fn drain(&mut self) -> impl Iterator<Item = (T, usize)> + '_ {
        self.0.drain()
    }
}

impl<T> Default for CountingSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Extend<T> for CountingSet<T>
where
    T: Hash + Eq,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for elem in iter {
            self.insert(elem);
        }
    }
}
