use std::hash::Hash;
use std::num::NonZeroU32;
use std::slice;
use std::sync::Arc;

use game_common::collections::scratch_buffer::ScratchBuffer;
use game_tracing::trace_span;
use hashbrown::HashMap;

use crate::backend::allocator::{BufferAlloc, GeneralPurposeAllocator, TextureAlloc, UsageFlags};
use crate::backend::descriptors::AllocatedDescriptorSet;
use crate::backend::vulkan::{self, CommandEncoder, TextureView};
use crate::backend::{
    AccessFlags, BufferBarrier, CopyBuffer, DrawIndexedIndirectCommand, DrawIndirectCommand,
    MemoryTypeFlags, PipelineBarriers, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, TextureBarrier, TextureViewDescriptor, TimestampPipelineStage,
    WriteDescriptorBinding, WriteDescriptorResource, WriteDescriptorResources,
};

use super::commands::{Command, CommandRef, ComputeCommand, ComputePassCmd, RenderPassCmd};
use super::queries::{ManagedQueryPool, QueryObject, QueryPoolSet};
use super::resources::DescriptorSetResource;
use super::scheduler::{Barrier, Step};
use super::{
    BufferId, DeletionEvent, DescriptorSetId, DrawCall, DrawCmd, PipelineId, ResourceId, Resources,
    SamplerId, TextureId,
};

#[derive(Debug)]
pub(super) struct Executor {
    allocator: GeneralPurposeAllocator,
    buffers: HashMap<BufferId, BufferAlloc>,
    textures: HashMap<TextureId, TextureData>,
    descriptor_sets: HashMap<DescriptorSetId, DescriptorSetInner>,
}

impl Executor {
    pub fn new(allocator: GeneralPurposeAllocator) -> Self {
        Self {
            allocator,
            buffers: HashMap::new(),
            textures: HashMap::new(),
            descriptor_sets: HashMap::new(),
        }
    }

    pub fn execute(
        &mut self,
        resources: &Resources,
        steps: Vec<Step<&CommandRef<'_>, ResourceId>>,
        encoder: &mut CommandEncoder<'_>,
        queries: &QueryPoolSet,
    ) -> TemporaryResources {
        let _span = trace_span!("Executor::execute").entered();

        // Note that we are taking the Vec and dropping it
        // at the end of the execution. We do this instead of
        // reusing the allocated memory because there may sometimes
        // be big chunks of data to be uploaded and that would result
        // in significant memory waste for most frames. Reallocating the
        // Vec is relatively cheap.
        let staging_memory_pool = core::mem::take(&mut *resources.staging_memory.lock());

        let mut render_passes = 0;
        let mut compute_passes = 0;

        for step in &steps {
            match step {
                Step::Node(cmd) => match cmd.as_ref() {
                    Command::RenderPass(_) => {
                        render_passes += 1;
                    }
                    Command::ComputePass(_) => {
                        compute_passes += 1;
                    }
                    _ => (),
                },
                _ => (),
            }
        }

        // Two timestamps for each pass: 1 before and 1 after
        // plus two timestamps to measure the entire command buffer time.
        let query_count = NonZeroU32::new(render_passes * 2 + compute_passes * 2 + 2).unwrap();
        let mut query_pool = queries.get(query_count);

        let mut tmp = TemporaryResources::default();

        let mut barriers = Vec::new();

        unsafe {
            let index = query_pool.next_index(QueryObject::BeginCommands);
            encoder.write_timestamp_query(query_pool.pool(), index, TimestampPipelineStage::None);
        }

        for step in steps {
            // Batch all barrier steps together, then when the new step is not
            // a barrier emit all barriers at once.
            if !barriers.is_empty() && !step.is_barrier() {
                insert_barriers(self, &barriers, encoder);
                barriers.clear();
            }

            match step {
                Step::Node(cmd) => match cmd.as_ref() {
                    Command::CreateBuffer(cmd) => {
                        let buffer = self.allocator.create_buffer(&cmd.descriptor);
                        self.buffers.insert(cmd.id, buffer);
                    }
                    Command::CreateTexture(cmd) => {
                        let texture = match cmd.resource.lock().take() {
                            Some(texture) => TextureData::Physical(texture),
                            None => {
                                let texture = self
                                    .allocator
                                    .create_texture(&cmd.descriptor, UsageFlags::empty());
                                TextureData::Virtual(texture)
                            }
                        };

                        self.textures.insert(cmd.id, texture);
                    }
                    Command::CreateDescriptorSet(id) => {
                        let set = build_descriptor_set(self, resources, *id);

                        self.descriptor_sets.insert(*id, set);
                    }
                    Command::DestoryBuffer(id) => {
                        debug_assert!(self.buffers.contains_key(id));

                        self.buffers.remove(id);
                    }
                    Command::DestroyTexture(id) => {
                        debug_assert!(self.textures.contains_key(id));

                        self.textures.remove(id);
                    }
                    Command::DestroyDescriptorSet(id) => {
                        debug_assert!(self.descriptor_sets.contains_key(id));

                        self.descriptor_sets.remove(id);
                    }
                    Command::WriteBuffer(cmd) => {
                        let buffer = self.buffers.get_mut(&cmd.buffer).unwrap();

                        let data = &staging_memory_pool
                            [cmd.staging_memory_offset..cmd.staging_memory_offset + cmd.count];

                        buffer.map()[cmd.offset as usize..cmd.offset as usize + cmd.count]
                            .copy_from_slice(&data);

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
                    Command::CopyBufferToBuffer(cmd) => {
                        let src = self.buffers.get(&cmd.src).unwrap();
                        let dst = self.buffers.get(&cmd.dst).unwrap();

                        // SAFETY:
                        // We have inserted the proper barriers such that the source is
                        // `TRANSFER_READ` and the destination is `TRANSFER_WRITE`.
                        unsafe {
                            encoder.copy_buffer_to_buffer(
                                src.buffer(),
                                cmd.src_offset,
                                dst.buffer(),
                                cmd.dst_offset,
                                cmd.count.get(),
                            );
                        }

                        // Both buffers must be kept alive until the copy
                        // operation is complete.
                        tmp.buffers.insert(cmd.src);
                        tmp.buffers.insert(cmd.dst);
                    }
                    Command::CopyBufferToTexture(cmd) => {
                        let src = self.buffers.get(&cmd.src).unwrap();
                        let dst = self.textures.get(&cmd.dst).unwrap();

                        // SAFETY:
                        // We have inserted the proper barriers such that the source buffer
                        // is `TRANSFER_READ` and the texture mip is `TRANSFER_WRITE`.
                        unsafe {
                            encoder.copy_buffer_to_texture(
                                CopyBuffer {
                                    buffer: src.buffer(),
                                    offset: cmd.src_offset,
                                    layout: cmd.layout,
                                },
                                dst.texture(),
                                cmd.dst_mip_level,
                            );
                        }

                        // Both buffer and texture must be kept alive until the
                        // copy operation is complete.
                        tmp.buffers.insert(cmd.src);
                        tmp.textures.insert(cmd.dst);
                    }
                    Command::CopyTextureToTexture(cmd) => {
                        let src = self.textures.get(&cmd.src).unwrap();
                        let dst = self.textures.get(&cmd.dst).unwrap();

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
                    Command::TextureTransition(cmd) => {
                        // The texture is not explicitly used anywhere else, but
                        // it still must be kept alive for this frame since it
                        // is used in a barrier.
                        tmp.textures.insert(cmd.texture.id);
                    }
                    Command::RenderPass(cmd) => {
                        run_render_pass(self, resources, &mut tmp, cmd, encoder, &mut query_pool);
                    }
                    Command::ComputePass(cmd) => {
                        run_compute_pass(self, resources, &mut tmp, cmd, encoder, &mut query_pool);
                    }
                },
                Step::Barrier(barrier) => {
                    barriers.push(barrier);
                }
            }
        }

        // Flush any remaining barriers.
        if !barriers.is_empty() {
            insert_barriers(self, &barriers, encoder);
        }

        unsafe {
            let index = query_pool.next_index(QueryObject::EndCommands);
            encoder.write_timestamp_query(query_pool.pool(), index, TimestampPipelineStage::All);
        }

        tmp.query_pool = Some(query_pool);

        tmp
    }
}

fn run_render_pass(
    executor: &mut Executor,
    resources: &Resources,
    tmp: &mut TemporaryResources,
    cmd: &RenderPassCmd,
    encoder: &mut CommandEncoder<'_>,
    query_pool: &mut ManagedQueryPool,
) {
    let attachment_views = ScratchBuffer::new(
        cmd.color_attachments.len() + usize::from(cmd.depth_stencil_attachment.is_some()),
    );

    let mut color_attachments = Vec::new();
    for attachment in &cmd.color_attachments {
        let texture = executor.textures.get(&attachment.target.texture).unwrap();

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
        let texture = executor.textures.get(&attachment.texture).unwrap();

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

    // Record a timestamp before running the render pass.
    unsafe {
        let index = query_pool.next_index(QueryObject::BeginPass(cmd.name));
        encoder.write_timestamp_query(query_pool.pool(), index, TimestampPipelineStage::None);
    }

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
                let set = executor.descriptor_sets.get(id).unwrap();

                render_pass.bind_descriptor_set(*index, set.set.raw());

                // Descriptor set must be kept alive until the render pass
                // completes.
                tmp.descriptor_sets.insert(*id);
            }
            DrawCmd::SetIndexBuffer(id, format) => {
                let buffer = executor.buffers.get(id).unwrap();

                render_pass.bind_index_buffer(buffer.buffer_view(), *format);

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
                let buffer = executor.buffers.get(&call.buffer).unwrap();

                render_pass.draw_indirect(
                    buffer.buffer(),
                    call.offset,
                    call.count,
                    size_of::<DrawIndirectCommand>() as u32,
                );

                tmp.buffers.insert(call.buffer);
            }
            DrawCmd::Draw(DrawCall::DrawIndexedIndirect(call)) => {
                let buffer = executor.buffers.get(&call.buffer).unwrap();

                render_pass.draw_indexed_indirect(
                    buffer.buffer(),
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

    // Record a second timestamp after the render pass render pass.
    unsafe {
        let index = query_pool.next_index(QueryObject::EndPass(cmd.name));
        encoder.write_timestamp_query(query_pool.pool(), index, TimestampPipelineStage::Graphics);
    }

    tmp.texture_views.extend(attachment_views);
}

fn run_compute_pass(
    executor: &mut Executor,
    resources: &Resources,
    tmp: &mut TemporaryResources,
    cmd: &ComputePassCmd,
    encoder: &mut CommandEncoder<'_>,
    query_pool: &mut ManagedQueryPool,
) {
    // Record a timestamp before running the compute pass.
    unsafe {
        let index = query_pool.next_index(QueryObject::BeginPass(cmd.name));
        encoder.write_timestamp_query(query_pool.pool(), index, TimestampPipelineStage::None);
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
                let set = executor.descriptor_sets.get(id).unwrap();

                compute_pass.bind_descriptor_set(*index, set.set.raw());

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

    // Record a second timestamp after the render compute render pass.
    unsafe {
        let index = query_pool.next_index(QueryObject::EndPass(cmd.name));
        encoder.write_timestamp_query(query_pool.pool(), index, TimestampPipelineStage::Compute);
    }
}

fn build_descriptor_set(
    executor: &Executor,
    resources: &Resources,
    id: DescriptorSetId,
) -> DescriptorSetInner {
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
                let buffer = executor.buffers.get(buffer).unwrap();
                let view = buffer.buffer_view();
                let view = buffer_views.insert(view);

                WriteDescriptorResource::UniformBuffer(slice::from_ref(view))
            }
            DescriptorSetResource::StorageBuffer(buffer) => {
                let buffer = executor.buffers.get(buffer).unwrap();
                let view = buffer.buffer_view();
                let view = buffer_views.insert(view);

                WriteDescriptorResource::StorageBuffer(slice::from_ref(view))
            }
            DescriptorSetResource::Texture(view) => {
                let texture = executor.textures.get(&view.texture).unwrap();
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
                    let texture = executor.textures.get(&view.texture).unwrap();

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

    let mut set = resources.descriptor_allocator.alloc(&layout.inner).unwrap();
    unsafe {
        set.raw_mut().update(&WriteDescriptorResources {
            bindings: &bindings,
        });
    }

    let mut textures = Vec::new();
    textures.extend(texture_views);
    for views in texture_array_views {
        textures.extend(views);
    }

    DescriptorSetInner { set, textures }
}

fn insert_barriers(
    executor: &mut Executor,
    barriers: &[Barrier<ResourceId>],
    encoder: &mut CommandEncoder<'_>,
) {
    let mut buffer_barriers = Vec::new();
    let mut texture_barriers = Vec::<TextureBarrier<'_>>::new();

    let mut last_texture_barrier_id = None;

    for barrier in barriers {
        match barrier.resource {
            ResourceId::Buffer(id) => {
                let buffer = executor.buffers.get(&id).unwrap();

                buffer_barriers.push(BufferBarrier {
                    buffer: buffer.buffer(),
                    offset: buffer.buffer_view().offset(),
                    size: buffer.buffer_view().len(),
                    src_access: barrier.src_access,
                    dst_access: barrier.dst_access,
                });
            }
            ResourceId::Texture(tex) => {
                let texture = executor.textures.get(&tex.id).unwrap();

                // If possible barriers are emitted in batches for each texture
                // with the mip levels ascending, so if a chain of consecutive
                // mip levels is modified we can batch all those barriers into one.
                if let Some(last_barrier) = texture_barriers.last_mut() {
                    if last_texture_barrier_id == Some(tex.id)
                        && last_barrier.src_access == barrier.src_access
                        && last_barrier.dst_access == barrier.dst_access
                        && last_barrier.base_mip_level + last_barrier.mip_levels == tex.mip_level
                    {
                        last_barrier.mip_levels += 1;

                        last_texture_barrier_id = Some(tex.id);
                        continue;
                    }
                }

                texture_barriers.push(TextureBarrier {
                    texture: texture.texture(),
                    src_access: barrier.src_access,
                    dst_access: barrier.dst_access,
                    base_mip_level: tex.mip_level,
                    mip_levels: 1,
                });

                last_texture_barrier_id = Some(tex.id);
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
    pub query_pool: Option<ManagedQueryPool>,
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

        if let Some(query_pool) = self.query_pool {
            unsafe {
                query_pool.release();
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

#[derive(Debug)]
struct DescriptorSetInner {
    set: AllocatedDescriptorSet,
    textures: Vec<TextureView<'static>>,
}

#[derive(Debug)]
pub enum TextureData {
    Physical(vulkan::Texture),
    Virtual(TextureAlloc),
}

impl TextureData {
    pub fn texture(&self) -> &vulkan::Texture {
        match &self {
            Self::Physical(tex) => tex,
            Self::Virtual(tex) => tex.texture(),
        }
    }
}
