//! Rendering API

pub mod executor;
pub mod queries;

mod commands;
mod resources;
mod scheduler;

use std::num::NonZeroU64;
use std::ops::{Bound, Range, RangeBounds};
use std::sync::Arc;

use bumpalo::Bump;
use commands::{
    Command, CommandStream, ComputeCommand, ComputePassCmd, CopyBufferToBuffer,
    CopyBufferToTexture, CopyTextureToTexture, RenderPassCmd, TextureTransition, WriteBuffer,
};
use crossbeam_queue::SegQueue;
use executor::TemporaryResources;
use game_common::cell::UnsafeRefCell;
use game_common::collections::vec_map::VecMap;
use game_common::components::Color;
use game_common::utils::exclusive::Exclusive;
use game_tracing::trace_span;
use glam::UVec2;
use hashbrown::{HashMap, HashSet};
use parking_lot::Mutex;
use queries::QueryPoolSet;
use resources::{
    BufferId, BufferInner, DescriptorSetId, DescriptorSetInner, DescriptorSetLayoutId,
    DescriptorSetLayoutInner, DescriptorSetResource, PipelineId, PipelineInner, RefCount,
    Resources, SamplerId, SamplerInner, TextureId, TextureInner,
};
use scheduler::{Node, Resource, ResourceMap, Scheduler};
use sharded_slab::Slab;

use crate::api::commands::{CreateBuffer, CreateTexture};
use crate::api::executor::Executor;
use crate::backend::allocator::{GeneralPurposeAllocator, MemoryManager, UsageFlags};
use crate::backend::descriptors::DescriptorSetAllocator;
use crate::backend::vulkan::{self, CommandEncoder, Device};
use crate::backend::{
    self, AccessFlags, AdapterMemoryProperties, AdapterProperties, BufferUsage, DepthStencilState,
    DescriptorType, DrawIndexedIndirectCommand, DrawIndirectCommand, Face, Features, FrontFace,
    ImageDataLayout, IndexFormat, LoadOp, PipelineStage, PrimitiveTopology, PushConstantRange,
    SamplerDescriptor, ShaderStage, ShaderStages, StoreOp, TextureDescriptor, TextureFormat,
    TextureUsage,
};
use crate::shader::{self, ShaderAccess, ShaderBinding};
use crate::statistics::Statistics;

pub use backend::DescriptorSetDescriptor as DescriptorSetLayoutDescriptor;

#[derive(Debug)]
pub struct CommandExecutor {
    resources: Arc<Resources>,
    // FIXME: The Mutex here is relatively low overhead, but
    // could be replaced by a concurrent queue with extra logic or similar.
    cmds: Mutex<CommandStream>,
    device: Device,
    adapter_props: AdapterProperties,
    allocator: Exclusive<Bump>,
    features: Features,
    scheduler: Scheduler,
    query_pools: QueryPoolSet,
    executor: Executor,
}

impl CommandExecutor {
    pub fn new(
        device: Device,
        memory_props: AdapterMemoryProperties,
        statistics: Arc<Statistics>,
        adapter_props: AdapterProperties,
    ) -> Self {
        let features = Features {
            mesh_shader: device.extensions().mesh_shader,
            task_shader: device.extensions().mesh_shader,
        };

        let allocator = GeneralPurposeAllocator::new(
            device.clone(),
            MemoryManager::new(device.clone(), memory_props),
            statistics,
        );

        Self {
            resources: Arc::new(Resources {
                pipelines: Slab::new(),
                buffers: Slab::new(),
                textures: Slab::new(),
                descriptor_sets: Slab::new(),
                descriptor_set_layouts: Slab::new(),
                descriptor_allocator: DescriptorSetAllocator::new(device.clone()),
                samplers: Slab::new(),
                deletion_queue: SegQueue::new(),
                staging_memory: Mutex::new(Vec::new()),
            }),
            cmds: Mutex::new(CommandStream::new()),
            query_pools: QueryPoolSet::new(device.clone()),
            device,
            adapter_props,
            allocator: Exclusive::new(Bump::new()),
            features,
            scheduler: Scheduler::new(),
            executor: Executor::new(allocator),
        }
    }

    pub fn queue(&self) -> CommandQueue<'_> {
        CommandQueue { executor: self }
    }

    pub fn execute(&mut self, encoder: &mut CommandEncoder<'_>) -> TemporaryResources {
        let _span = trace_span!("CommandExecutor::execute").entered();

        let allocator = self.allocator.get_mut();

        let cmd_stream = self.cmds.get_mut();

        let cmds = cmd_stream.cmd_refs();

        let mut steps = self
            .scheduler
            .schedule(&*self.resources, &*allocator, &cmds);
        allocator.reset();

        let prio_cmds = cmd_stream.priority_cmds();
        for (index, cmd) in prio_cmds.iter().enumerate() {
            steps.insert(index, scheduler::Step::Node(cmd));
        }

        let tmp = self
            .executor
            .execute(&self.resources, steps, encoder, &self.query_pools);

        self.cmds.get_mut().clear();

        tmp
    }

    pub fn destroy(&mut self, tmp: TemporaryResources) {
        tmp.destroy(&mut self.resources);
        self.cleanup();
    }

    fn cleanup(&mut self) {
        let _span = trace_span!("CommandExecutor::cleanup").entered();

        while let Some(cmd) = self.resources.deletion_queue.pop() {
            match cmd {
                DeletionEvent::Buffer(id) => {
                    self.resources.buffers.remove(id);

                    self.cmds
                        .lock()
                        .push(&self.resources, Command::DestoryBuffer(id));
                }
                DeletionEvent::Texture(id) => {
                    self.resources.textures.remove(id);

                    self.cmds
                        .lock()
                        .push(&self.resources, Command::DestroyTexture(id));
                }
                DeletionEvent::DescriptorSetLayout(id) => {
                    self.resources.descriptor_set_layouts.remove(id);
                }
                DeletionEvent::Pipeline(id) => {
                    self.resources.pipelines.remove(id);
                }
                DeletionEvent::Sampler(id) => {
                    self.resources.samplers.remove(id);
                }
                DeletionEvent::DescriptorSet(id) => {
                    let set = self.resources.descriptor_sets.take(id).unwrap();

                    self.cmds
                        .lock()
                        .push(&self.resources, Command::DestroyDescriptorSet(id));

                    for resource in set.bindings.into_values() {
                        match resource {
                            DescriptorSetResource::UniformBuffer(id)
                            | DescriptorSetResource::StorageBuffer(id) => {
                                let buffer = self.resources.buffers.get(id).unwrap();

                                if buffer.ref_count.decrement() {
                                    self.resources
                                        .deletion_queue
                                        .push(DeletionEvent::Buffer(id));
                                }
                            }
                            DescriptorSetResource::Texture(view) => {
                                let texture = self.resources.textures.get(view.texture).unwrap();

                                if texture.ref_count.decrement() {
                                    self.resources
                                        .deletion_queue
                                        .push(DeletionEvent::Texture(view.texture));
                                }
                            }
                            DescriptorSetResource::TextureArray(views) => {
                                for view in views {
                                    let texture =
                                        self.resources.textures.get(view.texture).unwrap();

                                    if texture.ref_count.decrement() {
                                        self.resources
                                            .deletion_queue
                                            .push(DeletionEvent::Texture(view.texture));
                                    }
                                }
                            }
                            DescriptorSetResource::Sampler(id) => {
                                let sampler = self.resources.samplers.get(id).unwrap();

                                if sampler.ref_count.decrement() {
                                    self.resources
                                        .deletion_queue
                                        .push(DeletionEvent::Sampler(id));
                                }
                            }
                        }
                    }

                    let layout = self
                        .resources
                        .descriptor_set_layouts
                        .get(set.layout)
                        .unwrap();
                    if layout.ref_count.decrement() {
                        self.resources
                            .deletion_queue
                            .push(DeletionEvent::DescriptorSetLayout(id));
                    }
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum ResourceId {
    Buffer(BufferId),
    Texture(TextureMip),
}

/// A queue for encoding rendering commands.
#[derive(Debug)]
pub struct CommandQueue<'a> {
    executor: &'a CommandExecutor,
}

impl<'a> CommandQueue<'a> {
    /// Returns the set of [`TextureUsage`] flags that the given [`TextureFormat`] supports.
    ///
    /// If the [`TextureFormat`] is not supported all all en empty set will be returned.
    pub fn supported_texture_usages(&self, format: TextureFormat) -> TextureUsage {
        self.executor
            .adapter_props
            .formats
            .get(&format)
            .copied()
            .unwrap_or(TextureUsage::empty())
    }

    pub fn features(&self) -> &Features {
        &self.executor.features
    }

    /// Creates a new [`Buffer`].
    pub fn create_buffer(&self, descriptor: &BufferDescriptor) -> Buffer {
        let id = self
            .executor
            .resources
            .buffers
            .insert(BufferInner {
                access: UnsafeRefCell::new(AccessFlags::empty()),
                ref_count: RefCount::new(),
            })
            .unwrap();

        self.executor.cmds.lock().push(
            &self.executor.resources,
            Command::CreateBuffer(CreateBuffer {
                id,
                descriptor: *descriptor,
            }),
        );

        Buffer {
            id,
            size: descriptor.size,
            usage: descriptor.usage,
            flags: descriptor.flags,
            resources: self.executor.resources.clone(),
        }
    }

    #[track_caller]
    pub fn create_buffer_init(&self, descriptor: &BufferInitDescriptor<'_>) -> Buffer {
        let buffer = self.create_buffer(&BufferDescriptor {
            size: descriptor.contents.len() as u64,
            usage: descriptor.usage | BufferUsage::TRANSFER_DST,
            flags: descriptor.flags,
        });
        self.write_buffer(&buffer, descriptor.contents);
        buffer
    }

    /// Writes `data` to the destination buffer.
    ///
    /// # Panics
    ///
    /// Panics if any following preconditions are violated:
    /// - The destination buffer does not have [`TRANSFER_DST`] set.
    /// - The destination buffer slice is smaller than `data`.
    ///
    /// [`TRANSFER_DST`]: BufferUsage::TRANSFER_DST
    #[track_caller]
    pub fn write_buffer<Dst>(&self, buffer: Dst, data: &[u8])
    where
        Dst: IntoBufferSlice,
    {
        self.write_buffer_inner(buffer.into_buffer_slice(), data);
    }

    #[track_caller]
    fn write_buffer_inner(&self, buffer: BufferSlice<'_>, data: &[u8]) {
        assert!(
            buffer.buffer.usage.contains(BufferUsage::TRANSFER_DST),
            "Buffer cannot be written to: TRANSFER_DST not set",
        );

        let buffer_len = buffer.end - buffer.start;
        assert!(
            buffer_len as usize >= data.len(),
            "destination buffer slice ({}) is too small for write of {} bytes",
            buffer_len,
            data.len(),
        );

        let mut pool = self.executor.resources.staging_memory.lock();
        let staging_memory_offset = pool.len();
        pool.extend_from_slice(data);

        // If the buffer is host visible we can map and write
        // to it directly.
        if buffer.buffer.flags.contains(UsageFlags::HOST_VISIBLE) {
            // The destination buffer must be kept alive until
            // the memcpy is complete.
            buffer.buffer.increment_ref_count();

            self.executor.cmds.lock().push(
                &self.executor.resources,
                Command::WriteBuffer(WriteBuffer {
                    buffer: buffer.buffer.id,
                    offset: buffer.start,
                    staging_memory_offset,
                    count: data.len(),
                }),
            );
        } else {
            // Otherwise we cannot access the buffer directly and
            // need to go through a host visible staging buffer.
            let staging_buffer = self.create_buffer(&BufferDescriptor {
                size: data.len() as u64,
                usage: BufferUsage::TRANSFER_SRC,
                flags: UsageFlags::HOST_VISIBLE,
            });

            debug_assert!(buffer.buffer.size >= staging_buffer.size);

            // The staging buffer must be kept alive until
            // the memcpy is complete.
            staging_buffer.increment_ref_count();

            // Write the data into the staging buffer.
            self.executor.cmds.lock().push(
                &self.executor.resources,
                Command::WriteBuffer(WriteBuffer {
                    buffer: staging_buffer.id,
                    offset: 0,
                    staging_memory_offset,
                    count: data.len(),
                }),
            );

            self.copy_buffer_to_buffer(&staging_buffer, buffer);
        }
    }

    #[track_caller]
    pub fn create_texture(&self, descriptor: &TextureDescriptor) -> Texture {
        assert!(
            !descriptor.usage.is_empty(),
            "TextureUsage flags must not be empty",
        );
        assert!(
            self.supported_texture_usages(descriptor.format)
                .contains(descriptor.usage),
            "unsupported texture usages: {:?} (supported are {:?})",
            descriptor.usage,
            self.supported_texture_usages(descriptor.format),
        );

        assert!(
            descriptor.size.x != 0 && descriptor.size.y != 0,
            "texture size must not be zero (size is {})",
            descriptor.size
        );

        let supported_usages = self
            .executor
            .adapter_props
            .formats
            .get(&descriptor.format)
            .copied()
            .unwrap_or(TextureUsage::empty());
        if supported_usages.contains(descriptor.usage) {}

        let mut mip_access = Vec::with_capacity(descriptor.mip_levels as usize);
        for _ in 0..descriptor.mip_levels {
            mip_access.push(UnsafeRefCell::new(AccessFlags::empty()));
        }

        let id = self
            .executor
            .resources
            .textures
            .insert(TextureInner {
                mip_access,
                ref_count: RefCount::new(),
            })
            .unwrap();

        self.executor.cmds.lock().push(
            &self.executor.resources,
            Command::CreateTexture(CreateTexture {
                id,
                descriptor: *descriptor,
                resource: Mutex::new(None),
            }),
        );

        Texture {
            id,
            size: descriptor.size,
            format: descriptor.format,
            usage: descriptor.usage,
            mip_levels: descriptor.mip_levels,
            manually_managed: false,
            resources: self.executor.resources.clone(),
        }
    }

    /// Imports an external texture.
    ///
    /// The [`TextureDescriptor`] must represent the texture layout of the external texture.
    /// The [`AccessFlags`] must represent the state of the texture at the current time.
    ///
    /// All texture subresources (texel regions, mips) must be in the same state specified by the
    /// [`AccessFlags`].
    ///
    /// # Safety
    ///
    /// The passed [`TextureDescriptor`] and [`AccessFlags`] must be valid and represent the state
    /// of the imported texture.
    pub(crate) unsafe fn import_texture(
        &mut self,
        texture: vulkan::Texture,
        descriptor: TextureDescriptor,
        access: AccessFlags,
    ) -> Texture {
        debug_assert_eq!(texture.size(), descriptor.size);
        debug_assert_eq!(texture.format(), descriptor.format);
        debug_assert_eq!(texture.mip_levels(), descriptor.mip_levels);

        let mut mip_access = Vec::with_capacity(descriptor.mip_levels as usize);
        for _ in 0..descriptor.mip_levels {
            mip_access.push(UnsafeRefCell::new(access));
        }

        let id = self
            .executor
            .resources
            .textures
            .insert(TextureInner {
                mip_access,
                ref_count: RefCount::new(),
            })
            .unwrap();

        self.executor.cmds.lock().push(
            &self.executor.resources,
            Command::CreateTexture(CreateTexture {
                id,
                descriptor,
                resource: Mutex::new(Some(texture)),
            }),
        );

        Texture {
            id,
            size: descriptor.size,
            format: descriptor.format,
            usage: descriptor.usage,
            mip_levels: descriptor.mip_levels,
            manually_managed: true,
            resources: self.executor.resources.clone(),
        }
    }

    pub(crate) fn remove_imported_texture(&self, texture: Texture) {
        debug_assert!(texture.manually_managed);

        let texture = self.executor.resources.textures.take(texture.id).unwrap();
        if !texture.ref_count.decrement() {
            panic!("Texture is still in use");
        }
    }

    /// Writes data into a texture.
    ///
    /// The physical layout of `data` is specified in [`ImageDataLayout`].
    ///
    /// # Panics
    ///
    /// Panics if any of the following preconditions are violated:
    /// - The texture does not have [`TRANSFER_DST`] set.
    /// - The selected mip level is out of bounds.
    /// - The number of bytes in `data` is not equal to the number of the texture region.
    ///
    /// [`TRANSFER_DST`]: TextureUsage::TRANSFER_DST
    #[track_caller]
    pub fn write_texture(&self, texture: TextureRegion<'_>, data: &[u8], layout: ImageDataLayout) {
        assert!(
            texture.texture.usage.contains(TextureUsage::TRANSFER_DST),
            "Texture cannot be written to: TRANSFER_DST usage not set",
        );

        assert!(texture.mip_level < texture.texture.mip_levels);

        let mip_size = texture.texture.size >> texture.mip_level;
        assert_eq!(layout.format.storage_size(mip_size), data.len());

        let staging_buffer = self.create_buffer_init(&BufferInitDescriptor {
            contents: data,
            usage: BufferUsage::TRANSFER_SRC,
            flags: UsageFlags::HOST_VISIBLE,
        });

        self.copy_buffer_to_texture(&staging_buffer, texture, layout);
    }

    /// Copies bytes from a source to a destination buffer.
    ///
    /// # Panics
    ///
    /// Panics if any of the following preconditions are violated:
    /// - The source buffer does not have [`TRANSFER_SRC`] set.
    /// - The destination buffer does not have [`TRANSFER_DST`] set.
    /// - The source slice has a different length than the destination slice.
    ///
    /// If source and desination buffers are the same:
    /// - The source and destination regions overlap.
    ///
    /// [`TRANSFER_SRC`]: BufferUsage::TRANSFER_SRC
    /// [`TRANSFER_DST`]: BufferUsage::TRANSFER_DST
    #[track_caller]
    pub fn copy_buffer_to_buffer<Src, Dst>(&self, src: Src, dst: Dst)
    where
        Src: IntoBufferSlice,
        Dst: IntoBufferSlice,
    {
        self.copy_buffer_to_buffer_inner(src.into_buffer_slice(), dst.into_buffer_slice());
    }

    #[track_caller]
    fn copy_buffer_to_buffer_inner(&self, src: BufferSlice<'_>, dst: BufferSlice<'_>) {
        assert!(
            src.buffer.usage.contains(BufferUsage::TRANSFER_SRC),
            "Buffer cannot be read from: TRANSFER_SRC usage not set",
        );
        assert!(
            dst.buffer.usage.contains(BufferUsage::TRANSFER_DST),
            "Buffer cannot be written to: TRANSFER_DST not set",
        );

        let src_len = src.end - src.start;
        let dst_len = dst.end - dst.start;
        assert_eq!(
            src_len, dst_len,
            "invalid buffer copy: source len {} must be equal to destination len {}",
            src_len, dst_len
        );

        // If copying within the same buffer we must ensure that
        // source and destination do not overlap.
        if src.buffer.id == dst.buffer.id {
            assert!(
                src.start >= dst.end || dst.start >= src.end,
                "if copying within the same buffer source {:?} and destination {:?} must not overlap",
                src.start..src.end,
                dst.start..dst.end,
            );
        }

        // Ensure that both buffers are big enough for the copy.
        // This invariant is guaranteed by the `BufferSlice` impl.
        debug_assert!(src.buffer.size >= src.start + src_len);
        debug_assert!(dst.buffer.size >= dst.start + dst_len);

        // We don't actually have to copy anything if the destination is
        // empty.
        let Some(count) = NonZeroU64::new(dst_len) else {
            return;
        };

        // The source and destination buffer must be kept alive
        // for this command.
        src.buffer.increment_ref_count();
        dst.buffer.increment_ref_count();

        self.executor.cmds.lock().push(
            &self.executor.resources,
            Command::CopyBufferToBuffer(CopyBufferToBuffer {
                src: src.buffer.id,
                src_offset: src.start,
                dst: dst.buffer.id,
                dst_offset: dst.start,
                count,
            }),
        );
    }

    pub fn copy_buffer_to_texture(
        &self,
        src: &Buffer,
        dst: TextureRegion<'_>,
        layout: ImageDataLayout,
    ) {
        assert!(
            src.usage.contains(BufferUsage::TRANSFER_SRC),
            "Buffer cannot be read from: TRANSFER_SRC usage not set",
        );
        assert!(
            dst.texture.usage.contains(TextureUsage::TRANSFER_DST),
            "Texture cannot be written to: TRANSFER_DST not set",
        );
        assert!(
            dst.mip_level < dst.texture.mip_levels,
            "Cannot write to mip level {}, only {} levels exist",
            dst.mip_level,
            dst.texture.mip_levels
        );

        // The source buffer and destination buffer must be kept alive
        // for this command.
        src.increment_ref_count();
        dst.texture.increment_ref_count();

        self.executor.cmds.lock().push(
            &self.executor.resources,
            Command::CopyBufferToTexture(CopyBufferToTexture {
                src: src.id,
                src_offset: 0,
                layout,
                dst: dst.texture.id,
                dst_mip_level: dst.mip_level,
            }),
        );
    }

    pub fn copy_texture_to_texture(&self, src: TextureRegion<'_>, dst: TextureRegion<'_>) {
        assert!(
            src.texture.usage.contains(TextureUsage::TRANSFER_SRC),
            "Texture cannot be read from: TRANSER_SRC not set"
        );
        assert!(
            dst.texture.usage.contains(TextureUsage::TRANSFER_DST),
            "Texture cannot be written to: TRANSFER_DST not set",
        );
        assert!(src.mip_level < src.texture.mip_levels);
        assert!(dst.mip_level < dst.texture.mip_levels);

        // The source and destination textures must be kept alive.
        src.texture.increment_ref_count();
        dst.texture.increment_ref_count();

        self.executor.cmds.lock().push(
            &self.executor.resources,
            Command::CopyTextureToTexture(CopyTextureToTexture {
                src: src.texture.id,
                src_mip_level: src.mip_level,
                dst: dst.texture.id,
                dst_mip_level: dst.mip_level,
            }),
        );
    }

    #[track_caller]
    pub fn create_descriptor_set(&self, descriptor: &DescriptorSetDescriptor<'_>) -> DescriptorSet {
        let mut bindings = VecMap::new();
        let mut num_buffers = 0;
        let mut num_samplers = 0;
        let mut num_textures = 0;
        let mut num_texture_arrays = 0;

        let layout = self
            .executor
            .resources
            .descriptor_set_layouts
            .get(descriptor.layout.id)
            .unwrap();

        let mut unset_descriptors: HashSet<_> =
            layout.bindings.values().map(|b| b.binding).collect();

        for entry in descriptor.entries {
            unset_descriptors.remove(&entry.binding);

            let layout_binding = layout.bindings.get(entry.binding);
            let layout_ty = layout_binding.map(|l| l.kind);

            match (&entry.resource, layout_ty) {
                (BindingResource::Buffer(_), Some(DescriptorType::Uniform)) => (),
                (BindingResource::Buffer(_), Some(DescriptorType::Storage)) => (),
                (BindingResource::Texture(_), Some(DescriptorType::Texture)) => (),
                (BindingResource::TextureArray(_), Some(DescriptorType::Texture)) => (),
                (BindingResource::Sampler(_), Some(DescriptorType::Sampler)) => (),
                _ => {
                    let found_type = match entry.resource {
                        BindingResource::Buffer(_) => "buffer",
                        BindingResource::Texture(_) => "texture",
                        BindingResource::TextureArray(_) => "texture array",
                        BindingResource::Sampler(_) => "sampler",
                    };

                    let expected_type = match layout_ty {
                        Some(DescriptorType::Uniform) => "uniform buffer",
                        Some(DescriptorType::Storage) => "storage buffer",
                        Some(DescriptorType::Texture) => "texture",
                        Some(DescriptorType::Sampler) => "sampler",
                        None => "none",
                    };

                    panic!(
                        "invalid descriptor at location {}: expected {}, found {}",
                        entry.binding, expected_type, found_type,
                    );
                }
            }

            let layout_ty = layout_ty.unwrap();

            let resource = match entry.resource {
                BindingResource::Buffer(buffer) if layout_ty == DescriptorType::Uniform => {
                    assert!(
                        buffer.usage.contains(BufferUsage::UNIFORM),
                        "Binding uniform buffer requires UNIFORM bit",
                    );

                    buffer.increment_ref_count();
                    num_buffers += 1;

                    DescriptorSetResource::UniformBuffer(buffer.id)
                }
                BindingResource::Buffer(buffer) if layout_ty == DescriptorType::Storage => {
                    assert!(
                        buffer.usage.contains(BufferUsage::STORAGE),
                        "Binding storage buffer requires STORAGE bit",
                    );

                    buffer.increment_ref_count();
                    num_buffers += 1;

                    DescriptorSetResource::StorageBuffer(buffer.id)
                }
                BindingResource::Buffer(_) => unreachable!(),
                BindingResource::Sampler(sampler) => {
                    sampler.increment_ref_count();
                    num_samplers += 1;

                    DescriptorSetResource::Sampler(sampler.id)
                }
                BindingResource::Texture(view) => {
                    assert!(
                        view.texture.usage.contains(TextureUsage::TEXTURE_BINDING),
                        "Texture cannot be bound to descriptor set: TEXTURE_BINDING not set",
                    );

                    view.texture.increment_ref_count();
                    num_textures += 1;

                    DescriptorSetResource::Texture(RawTextureView {
                        texture: view.texture.id,
                        base_mip_level: view.base_mip_level,
                        mip_levels: view.mip_levels,
                    })
                }
                BindingResource::TextureArray(views) => {
                    for view in views {
                        assert!(
                            view.texture.usage.contains(TextureUsage::TEXTURE_BINDING),
                            "Texture cannot be bound to descriptor set: TEXTURE_BINDING not set",
                        );
                    }

                    for view in views {
                        view.texture.increment_ref_count();
                    }

                    num_texture_arrays += 1;

                    DescriptorSetResource::TextureArray(
                        views
                            .into_iter()
                            .map(|view| RawTextureView {
                                texture: view.texture.id,
                                base_mip_level: view.base_mip_level,
                                mip_levels: view.mip_levels,
                            })
                            .collect(),
                    )
                }
            };

            bindings.insert(entry.binding, resource);
        }

        if !unset_descriptors.is_empty() {
            let text = unset_descriptors
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",");

            panic!("no descriptors at locations {} bound", text);
        }

        self.executor
            .resources
            .descriptor_set_layouts
            .get(descriptor.layout.id)
            .unwrap()
            .ref_count
            .increment();

        let id = self
            .executor
            .resources
            .descriptor_sets
            .insert(DescriptorSetInner {
                bindings,
                num_buffers,
                num_samplers,
                num_textures,
                num_texture_arrays,
                layout: descriptor.layout.id,
                ref_count: RefCount::new(),
            })
            .unwrap();

        self.executor
            .cmds
            .lock()
            .push(&self.executor.resources, Command::CreateDescriptorSet(id));

        DescriptorSet {
            id,
            resources: self.executor.resources.clone(),
        }
    }

    pub fn create_descriptor_set_layout(
        &self,
        descriptor: &DescriptorSetLayoutDescriptor<'_>,
    ) -> DescriptorSetLayout {
        let inner = self
            .executor
            .device
            .create_descriptor_layout(descriptor)
            .unwrap();
        let id = self
            .executor
            .resources
            .descriptor_set_layouts
            .insert(DescriptorSetLayoutInner {
                inner,
                ref_count: RefCount::new(),
                bindings: descriptor
                    .bindings
                    .iter()
                    .map(|v| (v.binding, *v))
                    .collect(),
            })
            .unwrap();

        DescriptorSetLayout {
            id,
            resources: self.executor.resources.clone(),
        }
    }

    #[track_caller]
    pub fn create_pipeline(&self, descriptor: &PipelineDescriptor<'_>) -> Pipeline {
        let layouts: Vec<_> = descriptor
            .descriptors
            .iter()
            .map(|layout| {
                self.executor
                    .resources
                    .descriptor_set_layouts
                    .get(layout.id)
                    .unwrap()
            })
            .collect();
        let descriptors: Vec<_> = layouts.iter().map(|layout| &layout.inner).collect();

        let validate_shader_binding = |binding: &ShaderBinding, stage: ShaderStage| {
            let get_descriptor_kind = |group: u32, binding: u32| -> Option<DescriptorType> {
                let descriptor = descriptors.get(group as usize)?;
                let binding = descriptor.bindings().get(binding as usize)?;
                Some(binding.kind)
            };

            let get_descriptor_vis = |group: u32, binding: u32| -> Option<ShaderStages> {
                let descriptor = descriptors.get(group as usize)?;
                let binding = descriptor.bindings().get(binding as usize)?;
                Some(binding.visibility)
            };

            assert!(
                get_descriptor_kind(binding.group, binding.binding)
                    .is_some_and(|kind| kind == binding.kind),
                "shader expects {:?} in location {:?}, but {:?} was provided",
                binding.kind,
                binding.location(),
                get_descriptor_kind(binding.group, binding.binding),
            );

            assert!(
                get_descriptor_vis(binding.group, binding.binding)
                    .is_some_and(|vis| vis.contains(stage.into())),
                "binding at {:?} must be visible in stage {:?}, but was only visible in stages {:?}",
                binding.location(),
                stage,
                get_descriptor_vis(binding.group, binding.binding),
            );
        };

        let mut bindings = BindingMap::default();
        for stage in descriptor.stages {
            match stage {
                PipelineStage::Vertex(stage) => {
                    let instance = stage.shader.instantiate(&shader::Options {
                        bindings: HashMap::new(),
                        stage: ShaderStage::Vertex,
                        entry_point: &stage.entry,
                    });

                    for binding in instance.bindings() {
                        validate_shader_binding(binding, ShaderStage::Vertex);

                        let mut access = AccessFlags::empty();
                        if binding.access.contains(ShaderAccess::READ) {
                            access |= AccessFlags::VERTEX_SHADER_READ;
                        }
                        if binding.access.contains(ShaderAccess::WRITE) {
                            access |= AccessFlags::VERTEX_SHADER_WRITE;
                        }

                        if !access.is_empty() {
                            bindings.insert(
                                binding.location().group,
                                binding.location().binding,
                                access,
                            );
                        }
                    }
                }
                PipelineStage::Fragment(stage) => {
                    let instance = stage.shader.instantiate(&shader::Options {
                        bindings: HashMap::new(),
                        stage: ShaderStage::Fragment,
                        entry_point: &stage.entry,
                    });

                    for binding in instance.bindings() {
                        validate_shader_binding(binding, ShaderStage::Fragment);

                        let mut access = AccessFlags::empty();
                        if binding.access.contains(ShaderAccess::READ) {
                            access |= AccessFlags::FRAGMENT_SHADER_READ;
                        }
                        if binding.access.contains(ShaderAccess::WRITE) {
                            access |= AccessFlags::FRAGMENT_SHADER_WRITE;
                        }

                        if !access.is_empty() {
                            bindings.insert(
                                binding.location().group,
                                binding.location().binding,
                                access,
                            );
                        }
                    }
                }
                PipelineStage::Task(stage) => {
                    let instance = stage.shader.instantiate(&shader::Options {
                        bindings: HashMap::new(),
                        stage: ShaderStage::Task,
                        entry_point: &stage.entry,
                    });

                    for binding in instance.bindings() {
                        validate_shader_binding(binding, ShaderStage::Task);

                        let mut access = AccessFlags::empty();
                        if binding.access.contains(ShaderAccess::READ) {
                            access |= AccessFlags::TASK_SHADER_READ;
                        }
                        if binding.access.contains(ShaderAccess::WRITE) {
                            access |= AccessFlags::TASK_SHADER_WRITE;
                        }

                        if !access.is_empty() {
                            bindings.insert(
                                binding.location().group,
                                binding.location().binding,
                                access,
                            );
                        }
                    }
                }
                PipelineStage::Mesh(stage) => {
                    let instance = stage.shader.instantiate(&shader::Options {
                        bindings: HashMap::new(),
                        stage: ShaderStage::Mesh,
                        entry_point: &stage.entry,
                    });

                    for binding in instance.bindings() {
                        validate_shader_binding(binding, ShaderStage::Mesh);

                        let mut access = AccessFlags::empty();
                        if binding.access.contains(ShaderAccess::READ) {
                            access |= AccessFlags::MESH_SHADER_READ;
                        }
                        if binding.access.contains(ShaderAccess::WRITE) {
                            access |= AccessFlags::MESH_SHADER_WRITE;
                        }

                        if !access.is_empty() {
                            bindings.insert(
                                binding.location().group,
                                binding.location().binding,
                                access,
                            );
                        }
                    }
                }
                PipelineStage::Compute(stage) => {
                    let instance = stage.shader.instantiate(&shader::Options {
                        bindings: HashMap::new(),
                        stage: ShaderStage::Compute,
                        entry_point: &stage.entry,
                    });

                    for binding in instance.bindings() {
                        validate_shader_binding(binding, ShaderStage::Compute);

                        let mut access = AccessFlags::empty();
                        if binding.access.contains(ShaderAccess::READ) {
                            access |= AccessFlags::COMPUTE_SHADER_READ;
                        }
                        if binding.access.contains(ShaderAccess::WRITE) {
                            access |= AccessFlags::COMPUTE_SHADER_WRITE;
                        }

                        if !access.is_empty() {
                            bindings.insert(
                                binding.location().group,
                                binding.location().binding,
                                access,
                            );
                        }
                    }
                }
            }
        }

        let inner = self
            .executor
            .device
            .create_pipeline(&backend::PipelineDescriptor {
                topology: descriptor.topology,
                cull_mode: descriptor.cull_mode,
                front_face: descriptor.front_face,
                descriptors: &descriptors,
                depth_stencil_state: descriptor.depth_stencil_state,
                stages: descriptor.stages,
                push_constant_ranges: descriptor.push_constant_ranges,
            })
            .unwrap();
        let id = self
            .executor
            .resources
            .pipelines
            .insert(PipelineInner {
                inner: Arc::new(inner),
                bindings,
                ref_count: RefCount::new(),
            })
            .unwrap();

        Pipeline {
            id,
            resources: self.executor.resources.clone(),
        }
    }

    pub fn create_sampler(&self, descriptor: &SamplerDescriptor) -> Sampler {
        let inner = self.executor.device.create_sampler(descriptor).unwrap();
        let id = self
            .executor
            .resources
            .samplers
            .insert(SamplerInner {
                inner,
                ref_count: RefCount::new(),
            })
            .unwrap();

        Sampler {
            id,
            resources: self.executor.resources.clone(),
        }
    }

    pub fn run_render_pass(&self, descriptor: &RenderPassDescriptor<'_>) -> RenderPass<'a, '_> {
        let color_attachments = descriptor
            .color_attachments
            .iter()
            .map(|a| {
                assert!(
                    a.target
                        .texture
                        .usage
                        .contains(TextureUsage::RENDER_ATTACHMENT),
                    "Texture cannot be used as color attachment: RENDER_ATTACHMENT not set",
                );

                self.executor
                    .resources
                    .textures
                    .get(a.target.texture.id)
                    .unwrap()
                    .ref_count
                    .increment();

                ColorAttachmentOwned {
                    target: RawTextureView {
                        texture: a.target.texture.id,
                        base_mip_level: a.target.base_mip_level,
                        mip_levels: a.target.mip_levels,
                    },
                    load_op: a.load_op,
                    store_op: a.store_op,
                }
            })
            .collect();

        let depth_stencil_attachment = descriptor.depth_stencil_attachment.map(|attachment| {
            self.executor
                .resources
                .textures
                .get(attachment.texture.id)
                .unwrap()
                .ref_count
                .increment();

            DepthStencilAttachmentOwned {
                texture: attachment.texture.id,
                load_op: attachment.load_op,
                store_op: attachment.store_op,
            }
        });

        RenderPass {
            name: descriptor.name,
            ctx: self,
            color_attachments,
            depth_stencil_attachment,
            cmds: Vec::new(),
            last_pipeline: None,
            last_index_buffer: None,
        }
    }

    pub fn run_compute_pass(&self, descriptor: &ComputePassDescriptor) -> ComputePass<'_> {
        ComputePass {
            name: descriptor.name,
            queue: self,
            last_pipeline: None,
            cmds: Vec::new(),
        }
    }

    /// Manually force a transition of the [`TextureRegion`] to the specified [`AccessFlags`].
    pub(crate) fn transition_texture(&self, texture: &TextureRegion<'_>, to: AccessFlags) {
        self.executor
            .resources
            .textures
            .get(texture.texture.id)
            .unwrap()
            .ref_count
            .increment();

        self.executor.cmds.lock().push(
            &self.executor.resources,
            Command::TextureTransition(TextureTransition {
                texture: TextureMip {
                    id: texture.texture.id,
                    mip_level: texture.mip_level,
                },
                access: to,
            }),
        );
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TextureRegion<'a> {
    pub texture: &'a Texture,
    pub mip_level: u32,
}

#[derive(Debug)]
pub struct DescriptorSet {
    id: DescriptorSetId,
    resources: Arc<Resources>,
}

impl Clone for DescriptorSet {
    fn clone(&self) -> Self {
        let set = self.resources.descriptor_sets.get(self.id).unwrap();
        set.ref_count.increment();

        Self {
            id: self.id,
            resources: self.resources.clone(),
        }
    }
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        let set = self.resources.descriptor_sets.get(self.id).unwrap();
        if set.ref_count.decrement() {
            self.resources
                .deletion_queue
                .push(DeletionEvent::DescriptorSet(self.id));
        }
    }
}

#[derive(Debug)]
pub struct Sampler {
    id: SamplerId,
    resources: Arc<Resources>,
}

impl Sampler {
    fn increment_ref_count(&self) {
        let sampler = self.resources.samplers.get(self.id).unwrap();
        sampler.ref_count.increment();
    }
}

impl Clone for Sampler {
    fn clone(&self) -> Self {
        let sampler = self.resources.samplers.get(self.id).unwrap();
        sampler.ref_count.increment();

        Self {
            id: self.id,
            resources: self.resources.clone(),
        }
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        let sampler = self.resources.samplers.get(self.id).unwrap();
        if sampler.ref_count.decrement() {
            self.resources
                .deletion_queue
                .push(DeletionEvent::Sampler(self.id));
        }
    }
}

#[derive(Clone, Debug)]
pub struct PipelineDescriptor<'a> {
    pub topology: PrimitiveTopology,
    pub front_face: FrontFace,
    pub cull_mode: Option<Face>,
    pub stages: &'a [PipelineStage<'a>],
    pub descriptors: &'a [&'a DescriptorSetLayout],
    pub push_constant_ranges: &'a [PushConstantRange],
    pub depth_stencil_state: Option<DepthStencilState>,
}

#[derive(Debug)]
pub struct DescriptorSetLayout {
    id: DescriptorSetLayoutId,
    resources: Arc<Resources>,
}

impl Clone for DescriptorSetLayout {
    fn clone(&self) -> Self {
        let layout = self.resources.descriptor_set_layouts.get(self.id).unwrap();
        layout.ref_count.increment();

        Self {
            id: self.id,
            resources: self.resources.clone(),
        }
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        let layout = self.resources.descriptor_set_layouts.get(self.id).unwrap();
        if layout.ref_count.decrement() {
            self.resources
                .deletion_queue
                .push(DeletionEvent::DescriptorSetLayout(self.id));
        }
    }
}

#[derive(Debug)]
pub struct Pipeline {
    id: PipelineId,
    resources: Arc<Resources>,
}

impl Clone for Pipeline {
    fn clone(&self) -> Self {
        let pipeline = self.resources.pipelines.get(self.id).unwrap();
        pipeline.ref_count.increment();

        Self {
            id: self.id,
            resources: self.resources.clone(),
        }
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        let pipeline = self.resources.pipelines.get(self.id).unwrap();
        if pipeline.ref_count.decrement() {
            self.resources
                .deletion_queue
                .push(DeletionEvent::Pipeline(self.id));
        }
    }
}

/// Tracks how a pipeline accesses each binding.
#[derive(Clone, Debug, Default)]
struct BindingMap {
    // Bindings are usually compact and continous so we can use direct
    // indexing and pad empty slots.
    // In most cases the number of empty slots is zero.
    groups: Vec<Vec<Option<AccessFlags>>>,
}

impl BindingMap {
    fn insert(&mut self, group: u32, binding: u32, flags: AccessFlags) {
        if group as usize >= self.groups.len() {
            self.groups.resize(group as usize + 1, Vec::new());
        }
        let group = &mut self.groups[group as usize];

        if binding as usize >= group.len() {
            group.resize(binding as usize + 1, None);
        }

        *group[binding as usize].get_or_insert_default() |= flags;
    }

    fn get(&self, group: u32, binding: u32) -> Option<AccessFlags> {
        self.groups
            .get(group as usize)?
            .get(binding as usize)?
            .as_ref()
            .copied()
    }
}

#[derive(Debug)]
pub struct Buffer {
    id: BufferId,
    size: u64,
    usage: BufferUsage,
    flags: UsageFlags,
    resources: Arc<Resources>,
}

impl Buffer {
    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn slice<T>(&self, range: T) -> BufferSlice<'_>
    where
        T: RangeBounds<u64>,
    {
        let start = match range.start_bound() {
            Bound::Included(index) => *index,
            Bound::Excluded(index) => *index + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(index) => *index + 1,
            Bound::Excluded(index) => *index,
            Bound::Unbounded => self.size,
        };

        assert!(start <= self.size && end <= self.size && start <= end);

        BufferSlice {
            buffer: self,
            start,
            end,
        }
    }

    fn increment_ref_count(&self) {
        let buffer = self.resources.buffers.get(self.id).unwrap();
        buffer.ref_count.increment();
    }
}

impl IntoBufferSlice for Buffer {
    fn into_buffer_slice(&self) -> BufferSlice<'_> {
        self.slice(..)
    }
}

impl<'a> IntoBufferSlice for &'a Buffer {
    fn into_buffer_slice(&self) -> BufferSlice<'_> {
        self.slice(..)
    }
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        let buffer = self.resources.buffers.get(self.id).unwrap();
        buffer.ref_count.increment();

        Self {
            id: self.id,
            size: self.size,
            usage: self.usage,
            flags: self.flags,
            resources: self.resources.clone(),
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        let buffer = self.resources.buffers.get(self.id).unwrap();
        if buffer.ref_count.decrement() {
            self.resources
                .deletion_queue
                .push(DeletionEvent::Buffer(self.id));
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BufferDescriptor {
    pub size: u64,
    pub usage: BufferUsage,
    pub flags: UsageFlags,
}

#[derive(Copy, Clone, Debug)]
pub struct BufferInitDescriptor<'a> {
    pub contents: &'a [u8],
    pub usage: BufferUsage,
    pub flags: UsageFlags,
}

pub trait IntoBufferSlice {
    fn into_buffer_slice(&self) -> BufferSlice<'_>;
}

#[derive(Copy, Clone, Debug)]
pub struct BufferSlice<'a> {
    buffer: &'a Buffer,
    start: u64,
    end: u64,
}

impl<'a> IntoBufferSlice for BufferSlice<'a> {
    fn into_buffer_slice(&self) -> BufferSlice<'_> {
        *self
    }
}

pub struct DescriptorSetDescriptor<'a> {
    pub layout: &'a DescriptorSetLayout,
    pub entries: &'a [DescriptorSetEntry<'a>],
}

pub struct DescriptorSetEntry<'a> {
    pub binding: u32,
    pub resource: BindingResource<'a>,
}

pub enum BindingResource<'a> {
    Buffer(&'a Buffer),
    Sampler(&'a Sampler),
    Texture(&'a TextureView),
    TextureArray(&'a [&'a TextureView]),
}

#[derive(Copy, Clone, Debug)]
enum DeletionEvent {
    Buffer(BufferId),
    Texture(TextureId),
    Sampler(SamplerId),
    Pipeline(PipelineId),
    DescriptorSet(DescriptorSetId),
    DescriptorSetLayout(DescriptorSetLayoutId),
}

#[derive(Copy, Clone, Debug)]
struct RawTextureView {
    texture: TextureId,
    base_mip_level: u32,
    mip_levels: u32,
}

impl RawTextureView {
    fn mips(&self) -> Range<u32> {
        self.base_mip_level..self.base_mip_level + self.mip_levels
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct TextureMip {
    id: TextureId,
    mip_level: u32,
}

#[derive(Debug)]
pub struct Texture {
    id: TextureId,
    size: UVec2,
    format: TextureFormat,
    usage: TextureUsage,
    mip_levels: u32,
    resources: Arc<Resources>,
    manually_managed: bool,
}

impl Texture {
    pub fn size(&self) -> UVec2 {
        self.size
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn mip_levels(&self) -> u32 {
        self.mip_levels
    }

    pub fn create_view(&self, descriptor: &TextureViewDescriptor) -> TextureView {
        let base_mip_level = descriptor.base_mip_level;
        let mip_levels = descriptor
            .mip_levels
            .unwrap_or(self.mip_levels - base_mip_level);

        assert_ne!(mip_levels, 0);
        assert!(base_mip_level + mip_levels <= self.mip_levels);

        TextureView {
            texture: self.clone(),
            base_mip_level,
            mip_levels,
        }
    }

    fn increment_ref_count(&self) {
        let texture = self.resources.textures.get(self.id).unwrap();
        texture.ref_count.increment();
    }
}

impl Clone for Texture {
    fn clone(&self) -> Self {
        if !self.manually_managed {
            let texture = self.resources.textures.get(self.id).unwrap();
            texture.ref_count.increment();
        }

        Self {
            id: self.id,
            size: self.size,
            format: self.format,
            usage: self.usage,
            mip_levels: self.mip_levels,
            resources: self.resources.clone(),
            manually_managed: self.manually_managed,
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if !self.manually_managed {
            let texture = self.resources.textures.get(self.id).unwrap();
            if texture.ref_count.decrement() {
                self.resources
                    .deletion_queue
                    .push(DeletionEvent::Texture(self.id));
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextureView {
    texture: Texture,
    base_mip_level: u32,
    mip_levels: u32,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct TextureViewDescriptor {
    pub base_mip_level: u32,
    pub mip_levels: Option<u32>,
}

pub struct RenderPass<'a, 'b> {
    name: &'static str,
    ctx: &'b CommandQueue<'a>,
    color_attachments: Vec<ColorAttachmentOwned>,
    depth_stencil_attachment: Option<DepthStencilAttachmentOwned>,
    cmds: Vec<DrawCmd>,
    // Exists purely for validation.
    last_pipeline: Option<PipelineId>,
    // Exists purely for validation.
    last_index_buffer: Option<(BufferId, IndexFormat)>,
}

impl<'a, 'b> RenderPass<'a, 'b> {
    pub fn set_pipeline(&mut self, pipeline: &Pipeline) {
        self.ctx
            .executor
            .resources
            .pipelines
            .get(pipeline.id)
            .unwrap()
            .ref_count
            .increment();

        self.cmds.push(DrawCmd::SetPipeline(pipeline.id));
        self.last_pipeline = Some(pipeline.id);
    }

    pub fn set_descriptor_set(&mut self, index: u32, descriptor_set: &'b DescriptorSet) {
        assert!(self.last_pipeline.is_some(), "Pipeline is not set");

        self.ctx
            .executor
            .resources
            .descriptor_sets
            .get(descriptor_set.id)
            .unwrap()
            .ref_count
            .increment();

        self.cmds
            .push(DrawCmd::SetDescriptorSet(index, descriptor_set.id));
    }

    pub fn set_push_constants(&mut self, stages: ShaderStages, offset: u32, data: &[u8]) {
        self.cmds
            .push(DrawCmd::SetPushConstants(data.to_vec(), stages, offset));
    }

    pub fn set_index_buffer(&mut self, buffer: &Buffer, format: IndexFormat) {
        assert!(self.last_pipeline.is_some(), "Pipeline is not set");
        assert!(
            buffer.usage.contains(BufferUsage::INDEX),
            "Buffer cannot be used as index buffer: INDEX not set",
        );

        self.ctx
            .executor
            .resources
            .buffers
            .get(buffer.id)
            .unwrap()
            .ref_count
            .increment();

        self.cmds.push(DrawCmd::SetIndexBuffer(buffer.id, format));
        self.last_index_buffer = Some((buffer.id, format));
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        assert!(self.last_pipeline.is_some(), "Pipeline is not set");

        self.cmds.push(DrawCmd::Draw(DrawCall::Draw(Draw {
            vertices,
            instances,
        })));
    }

    pub fn draw_indexed(&mut self, indices: Range<u32>, vertex_offset: i32, instances: Range<u32>) {
        assert!(self.last_pipeline.is_some(), "Pipeline is not set");
        let Some((buffer, format)) = self.last_index_buffer else {
            panic!("cannot call draw_indexed before binding a index buffer");
        };

        // The minimun index buffer size is:
        // (first_index + index_count) * index_size =
        // (indices.start + indices.end - indices.start) * index_size =
        // indices.end * index_size
        let Some(min_size) = indices.end.checked_mul(format.size().into()) else {
            panic!(
                "overflow computing index buffer size: indices={:?} format={:?}",
                indices, format
            );
        };

        let buffer = self.ctx.executor.resources.buffers.get(buffer).unwrap();
        // let buffer_size = unsafe { buffer.buffer.borrow().size() };
        // assert!(
        //     buffer_size >= min_size.into(),
        //     "index buffer of size {} is too small for indices={:?} format={:?}",
        //     buffer_size,
        //     indices,
        //     format,
        // );

        self.cmds
            .push(DrawCmd::Draw(DrawCall::DrawIndexed(DrawIndexed {
                indices,
                vertex_offset,
                instances,
            })));
    }

    /// Dispatches draw commands from the given `indirect_buffer`.
    ///
    /// The `indirect_buffer` must contain an array of [`DrawIndirectCommand`].
    ///
    /// # Panics
    ///
    /// Panics if any of the following preconditions are violated:
    /// - No pipeline was set.
    /// - `indirect_buffer` does not have the [`INDIRECT`] flag set.
    /// - `indirect_buffer` does not refer to a valid array of [`DrawIndirectCommand`].
    ///
    /// [`INDIRECT`]: BufferUsage::INDIRECT
    pub fn draw_indirect<T>(&mut self, indirect_buffer: &T)
    where
        T: IntoBufferSlice,
    {
        let indirect_buffer = indirect_buffer.into_buffer_slice();

        assert!(self.last_pipeline.is_some(), "Pipeline is not set");
        assert!(indirect_buffer.buffer.usage.contains(BufferUsage::INDIRECT));

        let len = indirect_buffer.end - indirect_buffer.start;
        assert_eq!(len % size_of::<DrawIndirectCommand>() as u64, 0);
        let count = len / size_of::<DrawIndirectCommand>() as u64;

        indirect_buffer.buffer.increment_ref_count();

        self.cmds
            .push(DrawCmd::Draw(DrawCall::DrawIndirect(DrawIndirect {
                buffer: indirect_buffer.buffer.id,
                offset: indirect_buffer.start,
                count: count as u32,
            })));
    }

    /// Dispatches indexed draw commands from the given `indirect_buffer`.
    ///
    /// The `indirect_buffer` must contain an array of [`DrawIndexedIndirectCommand`].
    ///
    /// # Panics
    ///
    /// Panics if any of the following preconditions are violated:
    /// - No pipeline was set.
    /// - `indirect_buffer` does not have the [`INDIRECT`] flag set.
    /// - `indirect_buffer` does not refer to a valid array of [`DrawIndexedIndirectCommand`].
    ///
    /// [`INDIRECT`]: BufferUsage::INDIRECT
    pub fn draw_indexed_indirect<T>(&mut self, indirect_buffer: &T)
    where
        T: IntoBufferSlice,
    {
        let indirect_buffer = indirect_buffer.into_buffer_slice();

        assert!(self.last_pipeline.is_some(), "Pipeline is not set");
        assert!(indirect_buffer.buffer.usage.contains(BufferUsage::INDIRECT));

        let len = indirect_buffer.end - indirect_buffer.start;
        assert_eq!(len % size_of::<DrawIndexedIndirectCommand>() as u64, 0);
        let count = len / size_of::<DrawIndexedIndirectCommand>() as u64;

        indirect_buffer.buffer.increment_ref_count();

        self.cmds.push(DrawCmd::Draw(DrawCall::DrawIndexedIndirect(
            DrawIndexedIndirect {
                buffer: indirect_buffer.buffer.id,
                offset: indirect_buffer.start,
                count: count as u32,
            },
        )));
    }

    pub fn draw_mesh_tasks(&mut self, x: u32, y: u32, z: u32) {
        assert!(self.last_pipeline.is_some(), "Pipeline is not set");

        self.cmds
            .push(DrawCmd::Draw(DrawCall::DrawMeshTasks(DrawMeshTasks {
                x,
                y,
                z,
            })));
    }
}

impl<'a, 'b> Drop for RenderPass<'a, 'b> {
    fn drop(&mut self) {
        self.ctx.executor.cmds.lock().push(
            &self.ctx.executor.resources,
            Command::RenderPass(RenderPassCmd {
                name: self.name,
                color_attachments: self.color_attachments.clone(),
                depth_stencil_attachment: self.depth_stencil_attachment.clone(),
                cmds: core::mem::take(&mut self.cmds),
            }),
        );
    }
}

#[derive(Debug)]
pub struct ComputePass<'a> {
    name: &'static str,
    queue: &'a CommandQueue<'a>,
    // Exists purely for validation.
    last_pipeline: Option<PipelineId>,
    cmds: Vec<ComputeCommand>,
}

impl<'a> ComputePass<'a> {
    pub fn set_pipeline(&mut self, pipeline: &Pipeline) {
        self.queue
            .executor
            .resources
            .pipelines
            .get(pipeline.id)
            .unwrap()
            .ref_count
            .increment();

        self.cmds.push(ComputeCommand::SetPipeline(pipeline.id));
        self.last_pipeline = Some(pipeline.id);
    }

    pub fn set_descriptor_set(&mut self, index: u32, descriptor_set: &DescriptorSet) {
        let pipeline = self.last_pipeline.expect("Pipeline is not set");

        let pipeline = self
            .queue
            .executor
            .resources
            .pipelines
            .get(pipeline)
            .unwrap();

        self.queue
            .executor
            .resources
            .descriptor_sets
            .get(descriptor_set.id)
            .unwrap()
            .ref_count
            .increment();

        self.cmds
            .push(ComputeCommand::SetDescriptorSet(index, descriptor_set.id));
    }

    pub fn set_push_constants(&mut self, stages: ShaderStages, offset: u32, data: &[u8]) {
        self.cmds.push(ComputeCommand::SetPushConstants(
            data.to_vec(),
            stages,
            offset,
        ));
    }

    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        assert!(self.last_pipeline.is_some(), "Pipeline is not set");

        self.cmds.push(ComputeCommand::Dispatch(x, y, z));
    }
}

impl<'a> Drop for ComputePass<'a> {
    fn drop(&mut self) {
        self.queue.executor.cmds.lock().push(
            &self.queue.executor.resources,
            Command::ComputePass(ComputePassCmd {
                name: self.name,
                cmds: core::mem::take(&mut self.cmds),
            }),
        );
    }
}

pub struct RenderPassDescriptor<'a> {
    pub name: &'static str,
    pub color_attachments: &'a [RenderPassColorAttachment<'a>],
    pub depth_stencil_attachment: Option<&'a DepthStencilAttachment<'a>>,
}

pub struct RenderPassColorAttachment<'a> {
    pub target: &'a TextureView,
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
    target: RawTextureView,
    load_op: LoadOp<Color>,
    store_op: StoreOp,
}

#[derive(Clone, Debug)]
struct DepthStencilAttachmentOwned {
    texture: TextureId,
    load_op: LoadOp<f32>,
    store_op: StoreOp,
}

#[derive(Clone, Debug)]
enum DrawCmd {
    SetPipeline(PipelineId),
    SetDescriptorSet(u32, DescriptorSetId),
    SetIndexBuffer(BufferId, IndexFormat),
    SetPushConstants(Vec<u8>, ShaderStages, u32),
    Draw(DrawCall),
}

#[derive(Clone, Debug)]
enum DrawCall {
    Draw(Draw),
    DrawIndexed(DrawIndexed),
    DrawIndirect(DrawIndirect),
    DrawIndexedIndirect(DrawIndexedIndirect),
    DrawMeshTasks(DrawMeshTasks),
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

#[derive(Clone, Debug)]
struct DrawIndirect {
    buffer: BufferId,
    offset: u64,
    count: u32,
}

#[derive(Clone, Debug)]
struct DrawIndexedIndirect {
    buffer: BufferId,
    offset: u64,
    count: u32,
}

#[derive(Copy, Clone, Debug)]
struct DrawMeshTasks {
    x: u32,
    y: u32,
    z: u32,
}

#[derive(Clone, Debug)]
pub struct ComputePassDescriptor {
    pub name: &'static str,
}
