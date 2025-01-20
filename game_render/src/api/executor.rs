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
    TextureBarrier, WriteDescriptorBinding, WriteDescriptorResource, WriteDescriptorResources,
};

use super::scheduler::{Barrier, Step};
use super::{
    Buffer, BufferId, Command, CopyBufferToTexture, DescriptorSet, DescriptorSetId, DrawCall,
    RenderPassCmd, ResourceId, Resources, Texture, TextureData, TextureId,
};

pub fn execute<'a, I>(
    resources: &mut Resources,
    steps: I,
    encoder: &mut CommandEncoder<'_>,
) -> TemporaryResources
where
    I: IntoIterator<Item = Step<&'a Command, ResourceId>>,
{
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
                write_buffer(resources, *id, data);
            }
            Step::Node(Command::CopyBufferToTexture(cmd)) => {
                copy_buffer_to_texture(&resources, &mut tmp, cmd, encoder);
            }
            Step::Node(Command::RenderPass(cmd)) => {
                run_render_pass(resources, &mut tmp, &cmd, encoder);
            }
            Step::Node(_) => (),
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

fn write_buffer(resources: &mut Resources, id: BufferId, data: &[u8]) {
    let buffer = resources.buffers.get_mut(id).unwrap();

    unsafe {
        buffer.buffer.map().copy_from_slice(&data);
    }
}

fn copy_buffer_to_texture(
    resources: &Resources,
    tmp: &mut TemporaryResources,
    cmd: &CopyBufferToTexture,
    encoder: &mut CommandEncoder<'_>,
) {
    let buffer = resources.buffers.get(cmd.buffer.id).unwrap();
    let texture = resources.textures.get(cmd.texture.id).unwrap();

    encoder.copy_buffer_to_texture(
        CopyBuffer {
            buffer: buffer.buffer.buffer(),
            offset: cmd.offset,
            layout: cmd.layout,
        },
        texture.data.texture(),
    );

    // Both buffer and texture must be kept alive until the
    // copy operation is complete.
    tmp.buffers.push(cmd.buffer.clone());
    tmp.textures.push(cmd.texture.clone());
}

fn run_render_pass(
    resources: &mut Resources,
    tmp: &mut TemporaryResources,
    cmd: &RenderPassCmd,
    encoder: &mut CommandEncoder<'_>,
) {
    for (_, set) in &cmd.descriptor_sets {
        if resources
            .descriptor_sets
            .get_mut(set.id)
            .unwrap()
            .descriptor_set
            .is_none()
        {
            build_descriptor_set(resources, set.id);
        }
    }

    let sets: Vec<_> = cmd
        .descriptor_sets
        .iter()
        .map(|(group, set)| (*group, resources.descriptor_sets.get(set.id).unwrap()))
        .collect();

    let attachment_views = ScratchBuffer::new(
        cmd.color_attachments.len() + usize::from(cmd.depth_stencil_attachment.is_some()),
    );

    let mut color_attachments = Vec::new();
    for attachment in &cmd.color_attachments {
        let texture = resources.textures.get(attachment.texture.id).unwrap();
        let physical_texture = match &texture.data {
            TextureData::Physical(data) => data,
            TextureData::Virtual(data) => data.texture(),
        };

        let view = attachment_views.insert(physical_texture.create_view());

        color_attachments.push(RenderPassColorAttachment {
            view,
            load_op: attachment.load_op,
            store_op: attachment.store_op,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        });
    }

    let depth_attachment = cmd.depth_stencil_attachment.as_ref().map(|attachment| {
        let texture = resources.textures.get(attachment.texture.id).unwrap();
        let physical_texture = match &texture.data {
            TextureData::Physical(data) => data,
            TextureData::Virtual(data) => data.texture(),
        };

        let view = attachment_views.insert(physical_texture.create_view());

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

    let pipeline = resources.pipelines.get(cmd.pipeline).unwrap();
    render_pass.bind_pipeline(&pipeline.inner);

    for (group, set) in sets {
        let set = set.descriptor_set.as_ref().unwrap().raw();
        render_pass.bind_descriptor_set(group, set);
    }

    for (data, stages, offset) in &cmd.push_constants {
        render_pass.set_push_constants(*stages, *offset, &data);
    }

    if let Some((buffer, format)) = &cmd.index_buffer {
        let buffer = resources.buffers.get(buffer.id).unwrap();
        render_pass.bind_index_buffer(buffer.buffer.buffer_view(), *format);
    }

    for call in cmd.draw_calls.iter().cloned() {
        match call {
            DrawCall::Draw(call) => {
                render_pass.draw(call.vertices, call.instances);
            }
            DrawCall::DrawIndexed(call) => {
                render_pass.draw_indexed(call.indices, call.vertex_offset, call.instances);
            }
        }
    }

    drop(render_pass);

    tmp.texture_views.extend(attachment_views);
    tmp.descriptor_sets
        .extend(cmd.descriptor_sets.values().cloned());
}

fn build_descriptor_set(resources: &mut Resources, id: DescriptorSetId) {
    let _span = trace_span!("build_descriptor_set").entered();

    let descriptor_set = resources.descriptor_sets.get_mut(id).unwrap();
    let layout = resources
        .descriptor_set_layouts
        .get(descriptor_set.layout.id)
        .unwrap();

    let mut bindings = Vec::new();

    let buffer_views = ScratchBuffer::new(descriptor_set.buffers.len());
    let texture_views = ScratchBuffer::new(descriptor_set.textures.len());
    let texture_array_views = ScratchBuffer::new(descriptor_set.texture_arrays.len());
    let texture_array_view_refs = ScratchBuffer::new(descriptor_set.texture_arrays.len());

    for (binding, buffer) in &descriptor_set.buffers {
        let buffer = resources.buffers.get(buffer.id).unwrap();

        let view = buffer_views.insert(buffer.buffer.buffer_view());

        match layout.inner.bindings()[*binding as usize].kind {
            DescriptorType::Uniform => {
                bindings.push(WriteDescriptorBinding {
                    binding: *binding,
                    resource: WriteDescriptorResource::UniformBuffer(&*view),
                });
            }
            DescriptorType::Storage => {
                bindings.push(WriteDescriptorBinding {
                    binding: *binding,
                    resource: WriteDescriptorResource::StorageBuffer(&*view),
                });
            }
            _ => unreachable!(),
        }
    }

    for (binding, texture) in &descriptor_set.textures {
        let texture = resources.textures.get(texture.id).unwrap();

        let physical_texture = match &texture.data {
            TextureData::Physical(data) => data,
            TextureData::Virtual(data) => data.texture(),
        };

        let view = texture_views.insert(physical_texture.create_view());

        bindings.push(WriteDescriptorBinding {
            binding: *binding,
            resource: WriteDescriptorResource::Texture(view),
        });
    }

    for (binding, sampler) in &descriptor_set.samplers {
        let sampler = resources.samplers.get(sampler.id).unwrap();
        bindings.push(WriteDescriptorBinding {
            binding: *binding,
            resource: WriteDescriptorResource::Sampler(&sampler.inner),
        });
    }

    for (binding, textures) in &descriptor_set.texture_arrays {
        let views = texture_array_views.insert(ScratchBuffer::new(textures.len()));

        for texture in textures {
            let texture = resources.textures.get(texture.id).unwrap();
            let physical_texture = match &texture.data {
                TextureData::Physical(data) => data,
                TextureData::Virtual(data) => data.texture(),
            };

            views.insert(physical_texture.create_view());
        }

        let view_refs = views.iter_mut().map(|v| &*v).collect::<Vec<_>>();
        let view_refs = texture_array_view_refs.insert(view_refs);

        bindings.push(WriteDescriptorBinding {
            binding: *binding,
            resource: WriteDescriptorResource::TextureArray(&*view_refs),
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
            ResourceId::Texture(id) => {
                let texture = resources.textures.get(id).unwrap();
                if barrier.src_access.is_empty()
                    && barrier.dst_access.is_readable()
                    && !barrier.dst_access.is_writable()
                {}
                texture_barriers.push(TextureBarrier {
                    texture: texture.data.texture(),
                    src_access: barrier.src_access,
                    dst_access: barrier.dst_access,
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
    descriptor_sets: Vec<DescriptorSet>,
    buffers: Vec<Buffer>,
    textures: Vec<Texture>,
}
