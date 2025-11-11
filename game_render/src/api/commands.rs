use std::num::NonZeroU64;

use bumpalo::Bump;
use game_common::utils::exclusive::Exclusive;
use hashbrown::{HashMap, HashSet};
use parking_lot::Mutex;

use crate::api::BufferDescriptor;
use crate::backend::{
    vulkan, AccessFlags, ImageDataLayout, LoadOp, ShaderStages, StoreOp, TextureDescriptor,
};

use super::resources::{DescriptorSetId, DescriptorSetResource, PipelineId};
use super::{
    BufferId, ColorAttachmentOwned, DepthStencilAttachmentOwned, DrawCall, DrawCmd, Node, Resource,
    ResourceId, Resources, TextureId, TextureMip,
};

#[derive(Debug)]
pub struct CommandStream {
    cmds: Vec<RecordedCommand>,
    /// Stores the access information for each [`Command`].
    ///
    /// This exists as a separate `Vec` to prevent constant allocations when inserting commands
    /// with a variable number of resource accesses.
    accesses: Vec<Resource<ResourceId>>,
    allocator: Exclusive<Bump>,
}

impl CommandStream {
    pub fn new() -> Self {
        Self {
            cmds: Vec::new(),
            accesses: Vec::new(),
            allocator: Exclusive::new(Bump::new()),
        }
    }

    /// Pushes a new [`Command`] to the end of this `CommandStream`.
    pub fn push(&mut self, resources: &Resources, cmd: Command) {
        let allocator = self.allocator.get_mut();
        let offset = self.accesses.len();
        cmd.write_accesses(resources, &mut self.accesses, &allocator);
        let count = self.accesses.len() - offset;
        allocator.reset();

        self.cmds.push(RecordedCommand {
            cmd,
            index: AcccessIndex { offset, count },
        });
    }

    /// Returns all commands recorded in this `CommandStream`.
    pub fn commands(&self) -> Vec<CommandRef<'_>> {
        self.cmds
            .iter()
            .map(|cmd| CommandRef {
                stream: self,
                cmd: &cmd.cmd,
                index: cmd.index,
            })
            .collect()
    }

    pub fn clear(&mut self) {
        self.cmds.clear();
        self.accesses.clear();
    }
}

#[derive(Debug)]
struct RecordedCommand {
    cmd: Command,
    index: AcccessIndex,
}

#[derive(Copy, Clone, Debug)]
struct AcccessIndex {
    offset: usize,
    count: usize,
}

/// Reference to a [`Command`] stored in a [`CommandStream`] with access to the computed resource
/// accesses.
#[derive(Copy, Clone)]
pub struct CommandRef<'a> {
    stream: &'a CommandStream,
    cmd: &'a Command,
    index: AcccessIndex,
}

impl<'a> Node for CommandRef<'a> {
    type ResourceId = ResourceId;

    fn resources(&self) -> &[Resource<ResourceId>] {
        let offset = self.index.offset;
        let count = self.index.count;

        // SAFETY:
        // - The `offset` and `count` are valid as written by `CommandStream::push`.
        unsafe { self.stream.accesses.get_unchecked(offset..offset + count) }
    }
}

impl AsRef<Command> for CommandRef<'_> {
    fn as_ref(&self) -> &Command {
        self.cmd
    }
}

impl std::fmt::Debug for CommandRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandRef")
            .field("index", &self.index)
            .field("cmd", &self.cmd)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub enum Command {
    WriteBuffer(WriteBuffer),
    CopyBufferToBuffer(CopyBufferToBuffer),
    CopyBufferToTexture(CopyBufferToTexture),
    CopyTextureToTexture(CopyTextureToTexture),
    TextureTransition(TextureTransition),
    RenderPass(RenderPassCmd),
    ComputePass(ComputePassCmd),
    CreateBuffer(CreateBuffer),
    CreateTexture(CreateTexture),
    CreateDescriptorSet(DescriptorSetId),
    DestoryBuffer(BufferId),
    DestroyTexture(TextureId),
    DestroyDescriptorSet(DescriptorSetId),
    ClearTexture(ClearTexture),
}

impl Command {
    fn write_accesses(
        &self,
        resources: &Resources,
        accesses: &mut Vec<Resource<ResourceId>>,
        alloc: &Bump,
    ) {
        match self {
            Self::WriteBuffer(cmd) => {
                accesses.push(Resource {
                    id: ResourceId::Buffer(cmd.buffer),
                    access: AccessFlags::TRANSFER_WRITE,
                });
            }
            Self::CopyBufferToBuffer(cmd) => {
                if cmd.src == cmd.dst {
                    accesses.push(Resource {
                        id: ResourceId::Buffer(cmd.src),
                        access: AccessFlags::TRANSFER_READ | AccessFlags::TRANSFER_WRITE,
                    });
                } else {
                    accesses.extend([
                        Resource {
                            id: ResourceId::Buffer(cmd.src),
                            access: AccessFlags::TRANSFER_READ,
                        },
                        Resource {
                            id: ResourceId::Buffer(cmd.dst),
                            access: AccessFlags::TRANSFER_WRITE,
                        },
                    ]);
                }
            }
            Self::CopyBufferToTexture(cmd) => {
                accesses.extend([
                    Resource {
                        id: ResourceId::Buffer(cmd.src),
                        access: AccessFlags::TRANSFER_READ,
                    },
                    Resource {
                        id: ResourceId::Texture(TextureMip {
                            id: cmd.dst,
                            mip_level: cmd.dst_mip_level,
                        }),
                        access: AccessFlags::TRANSFER_WRITE,
                    },
                ]);
            }
            Self::CopyTextureToTexture(cmd) => {
                if cmd.src == cmd.dst && cmd.src_mip_level == cmd.dst_mip_level {
                    accesses.push(Resource {
                        id: ResourceId::Texture(TextureMip {
                            id: cmd.src,
                            mip_level: cmd.src_mip_level,
                        }),
                        access: AccessFlags::TRANSFER_READ | AccessFlags::TRANSFER_WRITE,
                    });
                } else {
                    accesses.extend([
                        Resource {
                            id: ResourceId::Texture(TextureMip {
                                id: cmd.src,
                                mip_level: cmd.src_mip_level,
                            }),
                            access: AccessFlags::TRANSFER_READ,
                        },
                        Resource {
                            id: ResourceId::Texture(TextureMip {
                                id: cmd.dst,
                                mip_level: cmd.dst_mip_level,
                            }),
                            access: AccessFlags::TRANSFER_WRITE,
                        },
                    ]);
                }
            }
            Self::TextureTransition(cmd) => {
                accesses.push(Resource {
                    id: ResourceId::Texture(cmd.texture),
                    access: cmd.access,
                });
            }
            Self::ClearTexture(cmd) => {
                accesses.push(Resource {
                    id: ResourceId::Texture(TextureMip {
                        id: cmd.id,
                        mip_level: cmd.mip_level,
                    }),
                    access: AccessFlags::TRANSFER_WRITE,
                });
            }
            Self::RenderPass(cmd) => {
                let mut access_flags = HashMap::<ResourceId, AccessFlags, _, &Bump>::new_in(alloc);
                // The same descriptor set may get bound multiple times,
                // but this has no effect on the access flags.
                // As such it is cheaper to track visited descriptor sets
                // and skip duplicate bindings.
                let mut visited_descriptor_sets = HashSet::new_in(alloc);

                let mut pipeline = None;

                for cmd in &cmd.cmds {
                    match cmd {
                        DrawCmd::SetPipeline(id) => {
                            pipeline = Some(resources.pipelines.get(*id).unwrap());
                        }
                        DrawCmd::SetIndexBuffer(buffer, _) => {
                            *access_flags.entry(ResourceId::Buffer(*buffer)).or_default() |=
                                AccessFlags::INDEX;
                        }
                        DrawCmd::SetDescriptorSet(group, id) => {
                            let Some(pipeline) = &pipeline else {
                                continue;
                            };

                            if visited_descriptor_sets.contains(id) {
                                continue;
                            }
                            visited_descriptor_sets.insert(id);

                            let descriptor_set = resources.descriptor_sets.get(*id).unwrap();

                            for (binding, resource) in &descriptor_set.bindings {
                                match resource {
                                    DescriptorSetResource::UniformBuffer(buffer)
                                    | DescriptorSetResource::StorageBuffer(buffer) => {
                                        if let Some(access) = pipeline.bindings.get(*group, binding)
                                        {
                                            *access_flags
                                                .entry(ResourceId::Buffer(*buffer))
                                                .or_default() |= access;
                                        }
                                    }
                                    DescriptorSetResource::SampledTexture(views)
                                    | DescriptorSetResource::StorageTexture(views) => {
                                        if let Some(access) = pipeline.bindings.get(*group, binding)
                                        {
                                            for view in views {
                                                for mip in view.mips() {
                                                    *access_flags
                                                        .entry(ResourceId::Texture(TextureMip {
                                                            id: view.texture,
                                                            mip_level: mip,
                                                        }))
                                                        .or_default() |= access;
                                                }
                                            }
                                        }
                                    }
                                    DescriptorSetResource::Sampler(_) => (),
                                }
                            }
                        }
                        DrawCmd::SetPushConstants(_, _, _) => (),
                        DrawCmd::Draw(DrawCall::Draw(_)) => (),
                        DrawCmd::Draw(DrawCall::DrawIndexed(_)) => (),
                        DrawCmd::Draw(DrawCall::DrawIndirect(call)) => {
                            *access_flags
                                .entry(ResourceId::Buffer(call.buffer))
                                .or_default() |= AccessFlags::INDIRECT;
                        }
                        DrawCmd::Draw(DrawCall::DrawIndexedIndirect(call)) => {
                            *access_flags
                                .entry(ResourceId::Buffer(call.buffer))
                                .or_default() |= AccessFlags::INDIRECT;
                        }
                        DrawCmd::Draw(DrawCall::DrawMeshTasks(_)) => (),
                    }
                }

                for attachment in &cmd.color_attachments {
                    let mut attachment_flags = AccessFlags::empty();

                    if matches!(attachment.load_op, LoadOp::Load) {
                        attachment_flags |= AccessFlags::COLOR_ATTACHMENT_READ;
                    }

                    if matches!(attachment.store_op, StoreOp::Store) {
                        attachment_flags |= AccessFlags::COLOR_ATTACHMENT_WRITE;
                    }

                    for mip in attachment.target.mips() {
                        *access_flags
                            .entry(ResourceId::Texture(TextureMip {
                                id: attachment.target.texture,
                                mip_level: mip,
                            }))
                            .or_default() |= attachment_flags;
                    }
                }

                if let Some(attachment) = &cmd.depth_stencil_attachment {
                    *access_flags
                        .entry(ResourceId::Texture(TextureMip {
                            id: attachment.texture,
                            mip_level: 0,
                        }))
                        .or_default() |=
                        AccessFlags::DEPTH_ATTACHMENT_READ | AccessFlags::DEPTH_ATTACHMENT_WRITE;
                }

                for (id, access) in access_flags {
                    // We should never require a resource without any flags.
                    // This could result in a texture transition into UNDEFINED
                    // which is always invalid.
                    debug_assert!(!access.is_empty());

                    accesses.push(Resource { id, access });
                }
            }
            Self::ComputePass(cmd) => {
                let mut access_flags = HashMap::<ResourceId, AccessFlags, _, &Bump>::new_in(alloc);
                // The same descriptor set may get bound multiple times,
                // but this has no effect on the access flags.
                // As such it is cheaper to track visited descriptor sets
                // and skip duplicate bindings.
                let mut visited_descriptor_sets = HashSet::new_in(alloc);

                let mut pipeline = None;

                for cmd in &cmd.cmds {
                    match cmd {
                        ComputeCommand::SetPipeline(id) => {
                            pipeline = Some(resources.pipelines.get(*id).unwrap());
                        }
                        ComputeCommand::SetDescriptorSet(group, id) => {
                            let Some(pipeline) = &pipeline else {
                                continue;
                            };

                            if visited_descriptor_sets.contains(id) {
                                continue;
                            }
                            visited_descriptor_sets.insert(id);

                            let descriptor_set = resources.descriptor_sets.get(*id).unwrap();

                            for (binding, resource) in &descriptor_set.bindings {
                                match resource {
                                    DescriptorSetResource::UniformBuffer(buffer)
                                    | DescriptorSetResource::StorageBuffer(buffer) => {
                                        if let Some(access) = pipeline.bindings.get(*group, binding)
                                        {
                                            *access_flags
                                                .entry(ResourceId::Buffer(*buffer))
                                                .or_default() |= access;
                                        }
                                    }
                                    DescriptorSetResource::SampledTexture(views)
                                    | DescriptorSetResource::StorageTexture(views) => {
                                        if let Some(access) = pipeline.bindings.get(*group, binding)
                                        {
                                            for view in views {
                                                for mip in view.mips() {
                                                    *access_flags
                                                        .entry(ResourceId::Texture(TextureMip {
                                                            id: view.texture,
                                                            mip_level: mip,
                                                        }))
                                                        .or_default() |= access;
                                                }
                                            }
                                        }
                                    }
                                    DescriptorSetResource::Sampler(_) => (),
                                }
                            }
                        }
                        ComputeCommand::SetPushConstants(_, _, _) => (),
                        ComputeCommand::Dispatch(_, _, _) => (),
                    }
                }

                for (id, access) in access_flags {
                    // We should never require a resource without any flags.
                    // This could result in a texture transition into UNDEFINED
                    // which is always invalid.
                    debug_assert!(!access.is_empty());

                    accesses.push(Resource { id, access });
                }
            }
            // We only need to "touch" the resource once to ensure
            // that this command gets placed before any uses.
            Self::CreateBuffer(cmd) => {
                accesses.push(Resource {
                    id: ResourceId::Buffer(cmd.id),
                    access: AccessFlags::empty(),
                });
            }
            // Destruction can happen anywhere in the graph since an destruction
            // command means the resource was not used this frame and will not
            // be used in future frames.
            Self::DestoryBuffer(_) => (),
            // We only need to "touch" the resource once to ensure
            // that this command gets placed before any uses.
            Self::CreateTexture(cmd) => {
                for mip_level in 0..cmd.descriptor.mip_levels {
                    accesses.push(Resource {
                        id: ResourceId::Texture(TextureMip {
                            id: cmd.id,
                            mip_level,
                        }),
                        access: AccessFlags::empty(),
                    });
                }
            }
            // Destruction can happen anywhere in the graph since an destruction
            // command means the resource was not used this frame and will not
            // be used in future frames.
            Self::DestroyTexture(_) => (),
            // Descriptor Sets are immediate and written from the CPU, i.e. they
            // have no requirements on dependencies.
            Self::CreateDescriptorSet(_) => (),
            Self::DestroyDescriptorSet(_) => (),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct WriteBuffer {
    pub buffer: BufferId,
    /// Offset in the buffer object.
    pub offset: u64,
    /// Starting index into the shared staging memory pool
    pub staging_memory_offset: usize,
    /// Number of bytes to copy.
    pub count: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct CopyBufferToBuffer {
    pub src: BufferId,
    pub src_offset: u64,
    pub dst: BufferId,
    pub dst_offset: u64,
    pub count: NonZeroU64,
}

#[derive(Copy, Clone, Debug)]
pub struct CopyBufferToTexture {
    pub src: BufferId,
    pub src_offset: u64,
    pub layout: ImageDataLayout,
    pub dst: TextureId,
    pub dst_mip_level: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct CopyTextureToTexture {
    pub src: TextureId,
    pub src_mip_level: u32,
    pub dst: TextureId,
    pub dst_mip_level: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct TextureTransition {
    pub texture: TextureMip,
    pub access: AccessFlags,
}

#[derive(Debug)]
pub struct RenderPassCmd {
    pub name: &'static str,
    pub color_attachments: Vec<ColorAttachmentOwned>,
    pub depth_stencil_attachment: Option<DepthStencilAttachmentOwned>,
    pub cmds: Vec<DrawCmd>,
}

#[derive(Debug)]
pub struct ComputePassCmd {
    pub name: &'static str,
    pub cmds: Vec<ComputeCommand>,
}

#[derive(Debug)]
pub enum ComputeCommand {
    SetPipeline(PipelineId),
    SetDescriptorSet(u32, DescriptorSetId),
    SetPushConstants(Vec<u8>, ShaderStages, u32),
    Dispatch(u32, u32, u32),
}

#[derive(Debug)]
pub struct CreateBuffer {
    pub id: BufferId,
    pub descriptor: BufferDescriptor,
}

#[derive(Debug)]
pub struct CreateTexture {
    pub id: TextureId,
    pub descriptor: TextureDescriptor,
    // TODO: Remove this mutex
    pub resource: Mutex<Option<vulkan::Texture>>,
}

#[derive(Copy, Clone, Debug)]
pub struct ClearTexture {
    pub id: TextureId,
    pub mip_level: u32,
    pub value: [u32; 4],
}
