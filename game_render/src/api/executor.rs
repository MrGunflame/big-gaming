use std::hash::Hash;
use std::slice;
use std::sync::Arc;

use game_common::collections::scratch_buffer::ScratchBuffer;
use game_tracing::trace_span;
use hashbrown::HashMap;

use crate::backend::allocator::BufferAlloc;
use crate::backend::vulkan::{self, CommandEncoder, TextureView};
use crate::backend::{
    AccessFlags, BufferBarrier, CopyBuffer, DrawIndexedIndirectCommand, DrawIndirectCommand,
    MemoryTypeFlags, PipelineBarriers, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, TextureBarrier, TextureViewDescriptor, WriteDescriptorBinding,
    WriteDescriptorResource, WriteDescriptorResources,
};

use super::commands::{
    Command, ComputeCommand, ComputePassCmd, CopyBufferToBuffer, CopyBufferToTexture,
    CopyTextureToTexture, RenderPassCmd, WriteBuffer,
};
use super::resources::DescriptorSetResource;
use super::scheduler::{Barrier, Step};
use super::{
    BufferId, DeletionEvent, DescriptorSetId, DrawCall, DrawCmd, PipelineId, ResourceId, Resources,
    SamplerId, TextureId,
};

pub(super) fn execute<I, T>(
    resources: &Resources,
    steps: I,
    encoder: &mut CommandEncoder<'_>,
) -> TemporaryResources
where
    I: IntoIterator<Item = Step<T, ResourceId>>,
    T: AsRef<Command>,
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
            Step::Node(cmd) => match cmd.as_ref() {
                Command::WriteBuffer(cmd) => {
                    write_buffer(resources, &mut tmp, cmd);
                }
                Command::CopyBufferToBuffer(cmd) => {
                    copy_buffer_to_buffer(resources, &mut tmp, cmd, encoder);
                }
                Command::CopyBufferToTexture(cmd) => {
                    copy_buffer_to_texture(resources, &mut tmp, cmd, encoder);
                }
                Command::CopyTextureToTexture(cmd) => {
                    copy_texture_to_texture(resources, &mut tmp, cmd, encoder);
                }
                Command::TextureTransition(cmd) => {
                    // The texture is not explicitly used anywhere else, but
                    // it still must be kept alive for this frame since it
                    // is used in a barrier.
                    tmp.textures.insert(cmd.texture.id);
                }
                Command::RenderPass(cmd) => {
                    run_render_pass(resources, &mut tmp, cmd, encoder);
                }
                Command::ComputePass(cmd) => {
                    run_compute_pass(resources, &mut tmp, cmd, encoder);
                }
            },
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

fn write_buffer(resources: &Resources, tmp: &mut TemporaryResources, cmd: &WriteBuffer) {
    let buffer = resources.buffers.get(cmd.buffer).unwrap();

    let mut buffer = unsafe { buffer.buffer.borrow_mut() };
    buffer.map()[cmd.offset as usize..cmd.offset as usize + cmd.data.len()]
        .copy_from_slice(&cmd.data);

    // If the memory of the buffer is not HOST_COHERENT it needs to
    // be flushed, otherwise it may never become visible to the device.
    // TODO: We should batch and do a single flush for all writes.
    if !buffer.flags().contains(MemoryTypeFlags::HOST_COHERENT) {
        unsafe {
            buffer.flush();
        }
    }

    tmp.buffers.insert(cmd.buffer);
}

fn copy_buffer_to_buffer(
    resources: &Resources,
    tmp: &mut TemporaryResources,
    cmd: &CopyBufferToBuffer,
    encoder: &mut CommandEncoder<'_>,
) {
    let src = resources.buffers.get(cmd.src).unwrap();
    let dst = resources.buffers.get(cmd.dst).unwrap();

    // SAFETY:
    // We have inserted the proper barriers such that the source is
    // `TRANSFER_READ` and the destination is `TRANSFER_WRITE`.
    unsafe {
        encoder.copy_buffer_to_buffer(
            src.buffer.borrow().buffer(),
            cmd.src_offset,
            dst.buffer.borrow().buffer(),
            cmd.dst_offset,
            cmd.count.get(),
        );
    }

    // Both buffers must be kept alive until the copy
    // operation is complete.
    tmp.buffers.insert(cmd.src);
    tmp.buffers.insert(cmd.dst);
}

fn copy_buffer_to_texture(
    resources: &Resources,
    tmp: &mut TemporaryResources,
    cmd: &CopyBufferToTexture,
    encoder: &mut CommandEncoder<'_>,
) {
    let buffer = resources.buffers.get(cmd.src).unwrap();
    let texture = resources.textures.get(cmd.dst).unwrap();

    // SAFETY:
    // We have inserted the proper barriers such that the source buffer
    // is `TRANSFER_READ` and the texture mip is `TRANSFER_WRITE`.
    unsafe {
        encoder.copy_buffer_to_texture(
            CopyBuffer {
                buffer: buffer.buffer.borrow().buffer(),
                offset: cmd.src_offset,
                layout: cmd.layout,
            },
            texture.texture(),
            cmd.dst_mip_level,
        );
    }

    // Both buffer and texture must be kept alive until the
    // copy operation is complete.
    tmp.buffers.insert(cmd.src);
    tmp.textures.insert(cmd.dst);
}

fn copy_texture_to_texture(
    resources: &Resources,
    tmp: &mut TemporaryResources,
    cmd: &CopyTextureToTexture,
    encoder: &mut CommandEncoder<'_>,
) {
    let src = resources.textures.get(cmd.src).unwrap();
    let dst = resources.textures.get(cmd.dst).unwrap();

    // Safety:
    // We have inserted the propert barriers such that the source texture
    // is only `TRANSFER_READ` and the destination texture is only
    // `TRANSFER_WRITE`.
    unsafe {
        encoder.copy_texture_to_texture(
            src.texture(),
            cmd.src_mip_level,
            dst.texture(),
            cmd.dst_mip_level,
        );
    }

    // Both textures need to be kept alive until the copy operation is complete.
    tmp.textures.insert(cmd.src);
    tmp.textures.insert(cmd.dst);
}

fn run_render_pass(
    resources: &Resources,
    tmp: &mut TemporaryResources,
    cmd: &RenderPassCmd,
    encoder: &mut CommandEncoder<'_>,
) {
    // Ensure all physical descriptor sets are created before
    // the render pass begins.
    for cmd in &cmd.cmds {
        match cmd {
            DrawCmd::SetDescriptorSet(_, id) => {
                let set = resources.descriptor_sets.get(*id).unwrap();
                if unsafe { set.descriptor_set.borrow().is_none() } {
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
        // Attachment must be kept alive until the render pass completes.
        tmp.textures.insert(attachment.target.texture);

        let view = attachment_views.insert(unsafe {
            texture
                .texture()
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
            access: AccessFlags::COLOR_ATTACHMENT_WRITE,
        });
    }

    let depth_attachment = cmd.depth_stencil_attachment.as_ref().map(|attachment| {
        let texture = resources.textures.get(attachment.texture).unwrap();

        // Attachment must be kept alive until the render pass completes.
        tmp.textures.insert(attachment.texture);

        // Depth stencil attachment can only be a single mip.
        let view = attachment_views.insert(unsafe {
            texture
                .texture()
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
            access: AccessFlags::DEPTH_ATTACHMENT_READ | AccessFlags::DEPTH_ATTACHMENT_WRITE,
        }
    });

    // SAFETY: We have inserted the necessary barrier so that all
    // color/depth attachments have the appropriate access flags set.
    let mut render_pass = unsafe {
        encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &color_attachments,
            depth_stencil_attachment: depth_attachment.as_ref(),
        })
    };

    let mut pipelines = Vec::new();

    for cmd in &cmd.cmds {
        match cmd {
            DrawCmd::SetPipeline(id) => {
                let pipeline = resources.pipelines.get(*id).unwrap();

                // We store the pipeline.inner `Arc` value in a Vec,
                // preventing it to get dropped for the duration of the following
                // commands.
                pipelines.push(pipeline.inner.clone());
                render_pass.bind_pipeline(unsafe { &*Arc::as_ptr(&pipeline.inner) });

                // Pipeline must be kept alive until the render pass
                // completes.
                tmp.pipelines.insert(*id);
            }
            DrawCmd::SetDescriptorSet(index, id) => {
                let set = resources.descriptor_sets.get(*id).unwrap();
                let set = unsafe { set.descriptor_set.borrow() };
                let set = set.as_ref().unwrap().raw();
                render_pass.bind_descriptor_set(*index, set);

                // Descriptor set must be kept alive until the render pass
                // completes.
                tmp.descriptor_sets.insert(*id);
            }
            DrawCmd::SetIndexBuffer(id, format) => {
                let buffer = resources.buffers.get(*id).unwrap();
                unsafe {
                    render_pass.bind_index_buffer(buffer.buffer.borrow().buffer_view(), *format);
                }

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
            DrawCmd::Draw(DrawCall::DrawIndirect(call)) => {
                let buffer = resources.buffers.get(call.buffer).unwrap();

                render_pass.draw_indirect(
                    unsafe { buffer.buffer.borrow().buffer() },
                    call.offset,
                    call.count,
                    size_of::<DrawIndirectCommand>() as u32,
                );

                tmp.buffers.insert(call.buffer);
            }
            DrawCmd::Draw(DrawCall::DrawIndexedIndirect(call)) => {
                let buffer = resources.buffers.get(call.buffer).unwrap();

                render_pass.draw_indexed_indirect(
                    unsafe { buffer.buffer.borrow().buffer() },
                    call.offset,
                    call.count,
                    size_of::<DrawIndexedIndirectCommand>() as u32,
                );

                tmp.buffers.insert(call.buffer);
            }
            DrawCmd::Draw(DrawCall::DrawMeshTasks(call)) => {
                render_pass.draw_mesh_tasks(call.x, call.y, call.z);
            }
        }
    }

    drop(render_pass);

    tmp.texture_views.extend(attachment_views);
}

fn run_compute_pass(
    resources: &Resources,
    tmp: &mut TemporaryResources,
    cmd: &ComputePassCmd,
    encoder: &mut CommandEncoder<'_>,
) {
    // Ensure all physical descriptor sets are created before
    // the render pass begins.
    for cmd in &cmd.cmds {
        match cmd {
            ComputeCommand::SetDescriptorSet(_, id) => {
                let set = resources.descriptor_sets.get(*id).unwrap();
                if unsafe { set.descriptor_set.borrow().is_none() } {
                    build_descriptor_set(resources, *id);
                }
            }
            _ => (),
        }
    }

    let mut compute_pass = encoder.begin_compute_pass();

    let mut pipelines = Vec::new();

    for cmd in &cmd.cmds {
        match cmd {
            ComputeCommand::SetPipeline(id) => {
                let pipeline = resources.pipelines.get(*id).unwrap();

                // We store the pipeline.inner `Arc` value in a Vec,
                // preventing it to get dropped for the duration of the following
                // commands.
                pipelines.push(pipeline.inner.clone());
                compute_pass.bind_pipeline(unsafe { &*Arc::as_ptr(&pipeline.inner) });

                // Pipeline must be kept alive until the render pass
                // completes.
                tmp.pipelines.insert(*id);
            }
            ComputeCommand::SetDescriptorSet(index, id) => {
                let set = resources.descriptor_sets.get(*id).unwrap();
                let set = unsafe { set.descriptor_set.borrow() };
                let set = set.as_ref().unwrap().raw();
                compute_pass.bind_descriptor_set(*index, set);

                // Descriptor set must be kept alive until the render pass
                // completes.
                tmp.descriptor_sets.insert(*id);
            }
            ComputeCommand::SetPushConstants(data, stages, offset) => {
                compute_pass.set_push_constants(*stages, *offset, data);
            }
            ComputeCommand::Dispatch(x, y, z) => {
                compute_pass.dispatch(*x, *y, *z);
            }
        }
    }

    drop(compute_pass);
}

fn build_descriptor_set(resources: &Resources, id: DescriptorSetId) {
    let _span = trace_span!("build_descriptor_set").entered();

    let descriptor_set = resources.descriptor_sets.get(id).unwrap();
    let layout = resources
        .descriptor_set_layouts
        .get(descriptor_set.layout)
        .unwrap();

    let mut bindings = Vec::new();

    let buffer_views = ScratchBuffer::new(descriptor_set.num_buffers as usize);
    let texture_views = ScratchBuffer::new(descriptor_set.num_textures as usize);
    let texture_array_views = ScratchBuffer::new(descriptor_set.num_texture_arrays as usize);

    for (binding, resource) in descriptor_set.bindings.iter() {
        let resource = match resource {
            DescriptorSetResource::UniformBuffer(buffer) => {
                let buffer = resources.buffers.get(*buffer).unwrap();
                let buffer =
                    unsafe { &*(buffer.buffer.borrow().buffer() as *const vulkan::Buffer) };
                let view = buffer.slice(..);
                let view = buffer_views.insert(view);

                WriteDescriptorResource::UniformBuffer(slice::from_ref(view))
            }
            DescriptorSetResource::StorageBuffer(buffer) => {
                let buffer = resources.buffers.get(*buffer).unwrap();
                let buffer =
                    unsafe { &*(buffer.buffer.borrow().buffer() as *const vulkan::Buffer) };
                let view = buffer.slice(..);
                let view = buffer_views.insert(view);

                WriteDescriptorResource::StorageBuffer(slice::from_ref(view))
            }
            DescriptorSetResource::Texture(view) => {
                let texture = resources.textures.get(view.texture).unwrap();
                let view = texture_views.insert(unsafe {
                    texture
                        .texture()
                        .create_view(&TextureViewDescriptor {
                            base_mip_level: view.base_mip_level,
                            mip_levels: view.mip_levels,
                        })
                        .make_static()
                });

                WriteDescriptorResource::Texture(slice::from_ref(view))
            }
            DescriptorSetResource::TextureArray(views) => {
                let texture_views = texture_array_views.insert(ScratchBuffer::new(views.len()));

                for view in views {
                    let texture = resources.textures.get(view.texture).unwrap();

                    texture_views.insert(unsafe {
                        texture
                            .texture()
                            .create_view(&TextureViewDescriptor {
                                base_mip_level: view.base_mip_level,
                                mip_levels: view.mip_levels,
                            })
                            .make_static()
                    });
                }

                WriteDescriptorResource::Texture(texture_views.as_slice())
            }
            DescriptorSetResource::Sampler(sampler) => {
                let sampler = resources.samplers.get(*sampler).unwrap();
                let sampler = unsafe { &*(&sampler.inner as *const vulkan::Sampler) };

                WriteDescriptorResource::Sampler(slice::from_ref(&sampler))
            }
        };

        bindings.push(WriteDescriptorBinding { binding, resource });
    }

    let mut set = unsafe { resources.descriptor_allocator.alloc(&layout.inner).unwrap() };
    unsafe {
        set.raw_mut().update(&WriteDescriptorResources {
            bindings: &bindings,
        });
    }

    let mut physical_texture_views = unsafe { descriptor_set.physical_texture_views.borrow_mut() };
    physical_texture_views.extend(texture_views);
    for texture_views in texture_array_views {
        physical_texture_views.extend(texture_views);
    }

    unsafe {
        *descriptor_set.descriptor_set.borrow_mut() = Some(set);
    }
}

fn insert_barriers(
    resources: &Resources,
    barriers: &[Barrier<ResourceId>],
    encoder: &mut CommandEncoder<'_>,
) {
    let mut buffer_barriers = Vec::new();
    let mut texture_barriers = Vec::new();

    let mut buffers = Vec::new();
    let mut textures = Vec::new();
    for barrier in barriers {
        match barrier.resource {
            ResourceId::Buffer(id) => {
                let buffer = resources.buffers.get(id).unwrap();
                unsafe {
                    let buffer = buffer.buffer.borrow();
                    buffers.push(&*(&*buffer as *const BufferAlloc));
                }
            }
            ResourceId::Texture(tex) => {
                let texture = resources.textures.get(tex.id).unwrap();
                textures.push(unsafe { &*(&*texture.texture() as *const vulkan::Texture) });
            }
        }
    }
    let mut buffers = buffers.iter();
    let mut textures = textures.iter();

    for barrier in barriers {
        match barrier.resource {
            ResourceId::Buffer(id) => {
                let buffer = buffers.next().unwrap();
                buffer_barriers.push(BufferBarrier {
                    buffer: buffer.buffer(),
                    offset: buffer.buffer_view().offset(),
                    size: buffer.buffer_view().len(),
                    src_access: barrier.src_access,
                    dst_access: barrier.dst_access,
                });
            }
            ResourceId::Texture(tex) => {
                let texture = textures.next().unwrap();
                texture_barriers.push(TextureBarrier {
                    texture: texture,
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
    texture_views: Vec<TextureView<'static>>,
    descriptor_sets: CountingSet<DescriptorSetId>,
    buffers: CountingSet<BufferId>,
    textures: CountingSet<TextureId>,
    pipelines: CountingSet<PipelineId>,
    samplers: CountingSet<SamplerId>,
}

impl TemporaryResources {
    pub(super) fn destroy(mut self, resources: &Resources) {
        drop(self.texture_views.drain(..));

        for (id, count) in self.buffers.drain() {
            let buffer = resources.buffers.get(id).unwrap();

            if buffer.ref_count.decrement_many(count) {
                resources.deletion_queue.push(DeletionEvent::Buffer(id));
            }
        }

        for (id, count) in self.textures.drain() {
            let texture = resources.textures.get(id).unwrap();

            if texture.ref_count.decrement_many(count) {
                resources.deletion_queue.push(DeletionEvent::Texture(id));
            }
        }

        for (id, count) in self.descriptor_sets.drain() {
            let set = resources.descriptor_sets.get(id).unwrap();

            if set.ref_count.decrement_many(count) {
                resources
                    .deletion_queue
                    .push(DeletionEvent::DescriptorSet(id));
            }
        }

        for (id, count) in self.samplers.drain() {
            let sampler = resources.samplers.get(id).unwrap();

            if sampler.ref_count.decrement_many(count) {
                resources.deletion_queue.push(DeletionEvent::Sampler(id));
            }
        }

        for (id, count) in self.pipelines.drain() {
            let pipeline = resources.pipelines.get(id).unwrap();

            if pipeline.ref_count.decrement_many(count) {
                resources.deletion_queue.push(DeletionEvent::Pipeline(id));
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
