use std::num::NonZeroU64;

use bumpalo::Bump;
use game_common::utils::exclusive::Exclusive;
use hashbrown::{HashMap, HashSet};

use crate::backend::{AccessFlags, ImageDataLayout};

use super::{
    BufferId, ColorAttachmentOwned, DepthStencilAttachmentOwned, DrawCmd, Node, Resource,
    ResourceId, Resources, TextureId, TextureMip,
};

#[derive(Debug)]
pub struct CommandStream {
    cmds: Vec<Command>,
    accesses: Vec<Resource<ResourceId>>,
    cmd_accesses: Vec<AcccessIndex>,
    allocator: Exclusive<Bump>,
}

impl CommandStream {
    pub fn new() -> Self {
        Self {
            cmds: Vec::new(),
            accesses: Vec::new(),
            cmd_accesses: Vec::new(),
            allocator: Exclusive::new(Bump::new()),
        }
    }

    pub fn push(&mut self, resources: &Resources, cmd: Command) {
        let allocator = self.allocator.get_mut();
        let offset = self.accesses.len();
        cmd.write_accesses(resources, &mut self.accesses, &allocator);
        let count = self.accesses.len() - offset;
        allocator.reset();

        self.cmds.push(cmd);
        self.cmd_accesses.push(AcccessIndex { offset, count });
    }

    pub fn cmd_refs(&self) -> Vec<CommandRef<'_>> {
        self.cmds
            .iter()
            .enumerate()
            .map(|(index, cmd)| CommandRef {
                stream: self,
                index,
                cmd,
            })
            .collect()
    }

    pub fn clear(&mut self) {
        self.cmds.clear();
        self.accesses.clear();
        self.cmd_accesses.clear();
    }
}

#[derive(Copy, Clone, Debug)]
struct AcccessIndex {
    offset: usize,
    count: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct CommandRef<'a> {
    stream: &'a CommandStream,
    index: usize,
    cmd: &'a Command,
}

impl<'a> Node for CommandRef<'a> {
    type ResourceId = ResourceId;

    fn resources(&self) -> &[Resource<ResourceId>] {
        let region = self.stream.cmd_accesses[self.index];
        &self.stream.accesses[region.offset..region.offset + region.count]
    }
}

impl AsRef<Command> for CommandRef<'_> {
    fn as_ref(&self) -> &Command {
        self.cmd
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
                            for (binding, buffer) in &descriptor_set.buffers {
                                if let Some(access) = pipeline.bindings.get(*group, *binding) {
                                    *access_flags
                                        .entry(ResourceId::Buffer(*buffer))
                                        .or_default() |= access;
                                }
                            }

                            for (binding, view) in &descriptor_set.textures {
                                if let Some(access) = pipeline.bindings.get(*group, *binding) {
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

                            for (binding, views) in &descriptor_set.texture_arrays {
                                if let Some(access) = pipeline.bindings.get(*group, *binding) {
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
                        }
                        DrawCmd::SetPushConstants(_, _, _) => (),
                        DrawCmd::Draw(_) => (),
                    }
                }

                for attachment in &cmd.color_attachments {
                    for mip in attachment.target.mips() {
                        *access_flags
                            .entry(ResourceId::Texture(TextureMip {
                                id: attachment.target.texture,
                                mip_level: mip,
                            }))
                            .or_default() |= AccessFlags::COLOR_ATTACHMENT_WRITE;
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
        }
    }
}

#[derive(Debug)]
pub struct WriteBuffer {
    pub buffer: BufferId,
    pub offset: u64,
    pub data: Vec<u8>,
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
    pub color_attachments: Vec<ColorAttachmentOwned>,
    pub depth_stencil_attachment: Option<DepthStencilAttachmentOwned>,
    pub cmds: Vec<DrawCmd>,
}
