//! Rendering API

mod commands;
pub mod executor;
mod scheduler;

use std::any::Any;
use std::num::NonZeroU64;
use std::ops::Range;
use std::sync::Arc;

use bumpalo::Bump;
use commands::{
    Command, CommandStream, CopyBufferToBuffer, CopyBufferToTexture, CopyTextureToTexture,
    RenderPassCmd, TextureTransition, WriteBuffer,
};
use crossbeam_queue::SegQueue;
use executor::TemporaryResources;
use game_common::collections::arena::{Arena, Key};
use game_common::components::Color;
use game_common::utils::exclusive::Exclusive;
use game_tracing::trace_span;
use glam::UVec2;
use hashbrown::HashMap;
use scheduler::{Node, Resource, ResourceMap, Scheduler};

use crate::backend::allocator::{
    BufferAlloc, GeneralPurposeAllocator, MemoryManager, TextureAlloc, UsageFlags,
};
use crate::backend::descriptors::{AllocatedDescriptorSet, DescriptorSetAllocator};
use crate::backend::shader::ShaderAccess;
use crate::backend::vulkan::{self, CommandEncoder, Device};
use crate::backend::{
    self, AccessFlags, AdapterMemoryProperties, AdapterProperties, BufferUsage, DepthStencilState,
    Face, FrontFace, ImageDataLayout, IndexFormat, LoadOp, PipelineStage, PrimitiveTopology,
    PushConstantRange, SamplerDescriptor, ShaderStage, ShaderStages, StoreOp, TextureDescriptor,
    TextureFormat, TextureUsage,
};
use crate::statistics::Statistics;

pub use backend::DescriptorSetDescriptor as DescriptorSetLayoutDescriptor;

type PipelineId = Key;
type BufferId = Key;
type TextureId = Key;
type DescriptorSetId = Key;
type DescriptorSetLayoutId = Key;
type SamplerId = Key;

#[derive(Debug)]
pub struct CommandExecutor {
    resources: Resources,
    cmds: CommandStream,
    device: Device,
    adapter_props: AdapterProperties,
    allocator: Exclusive<Bump>,
}

impl CommandExecutor {
    pub fn new(
        device: Device,
        memory_props: AdapterMemoryProperties,
        statistics: Arc<Statistics>,
        adapter_props: AdapterProperties,
    ) -> Self {
        Self {
            resources: Resources {
                pipelines: Arena::new(),
                buffers: Arena::new(),
                textures: Arena::new(),
                descriptor_sets: Arena::new(),
                descriptor_set_layouts: Arena::new(),
                allocator: GeneralPurposeAllocator::new(
                    device.clone(),
                    MemoryManager::new(device.clone(), memory_props),
                    statistics,
                ),
                descriptor_allocator: DescriptorSetAllocator::new(device.clone()),
                samplers: Arena::new(),
                lifecycle_events: Arc::new(SegQueue::new()),
            },
            cmds: CommandStream::new(),
            device,
            adapter_props,
            allocator: Exclusive::new(Bump::new()),
        }
    }

    pub fn queue(&mut self) -> CommandQueue<'_> {
        CommandQueue { executor: self }
    }

    pub fn execute(&mut self, encoder: &mut CommandEncoder<'_>) -> TemporaryResources {
        let _span = trace_span!("CommandExecutor::execute").entered();

        let allocator = self.allocator.get_mut();

        let cmds = self.cmds.cmd_refs();

        let mut scheduler = Scheduler {
            resources: &mut self.resources,
            allocator: &*allocator,
        };

        let steps = scheduler.schedule(&cmds);
        allocator.reset();
        let tmp = executor::execute(&mut self.resources, steps, encoder);
        self.cmds.clear();

        tmp
    }

    pub fn destroy(&mut self, tmp: TemporaryResources) {
        tmp.destroy(&mut self.resources);
        self.cleanup();
    }

    fn cleanup(&mut self) {
        let _span = trace_span!("CommandExecutor::cleanup").entered();

        while let Some(cmd) = self.resources.lifecycle_events.pop() {
            match cmd {
                LifecycleEvent::DestroyBufferHandle(id) => {
                    let buffer = self.resources.buffers.get_mut(id).unwrap();
                    buffer.ref_count -= 1;
                    if buffer.ref_count == 0 {
                        self.resources.buffers.remove(id).unwrap();
                    }
                }
                LifecycleEvent::DestroyTextureHandle(id) => {
                    let texture = self.resources.textures.get_mut(id).unwrap();
                    texture.ref_count -= 1;
                    if texture.ref_count == 0 {
                        self.resources.textures.remove(id).unwrap();
                    }
                }
                LifecycleEvent::DestroySamplerHandle(id) => {
                    let sampler = self.resources.samplers.get_mut(id).unwrap();
                    sampler.ref_count -= 1;
                    if sampler.ref_count == 0 {
                        self.resources.samplers.remove(id).unwrap();
                    }
                }
                LifecycleEvent::DestroyDescriptorSetHandle(id) => {
                    let set = self.resources.descriptor_sets.get_mut(id).unwrap();
                    set.ref_count -= 1;
                    if set.ref_count != 0 {
                        continue;
                    }

                    for (_, id) in &set.buffers {
                        self.resources
                            .lifecycle_events
                            .push(LifecycleEvent::DestroyBufferHandle(*id));
                    }

                    for (_, view) in &set.textures {
                        self.resources
                            .lifecycle_events
                            .push(LifecycleEvent::DestroyTextureHandle(view.texture));
                    }

                    for (_, views) in &set.texture_arrays {
                        for view in views {
                            self.resources
                                .lifecycle_events
                                .push(LifecycleEvent::DestroyTextureHandle(view.texture));
                        }
                    }

                    self.resources
                        .lifecycle_events
                        .push(LifecycleEvent::DestroyDescriptorSetLayoutHandle(set.layout));

                    self.resources.descriptor_sets.remove(id);
                }
                LifecycleEvent::DestroyPipelineHandle(id) => {
                    let pipeline = self.resources.pipelines.get_mut(id).unwrap();
                    pipeline.ref_count -= 1;
                    if pipeline.ref_count == 0 {
                        self.resources.pipelines.remove(id);
                    }
                }
                LifecycleEvent::DestroyDescriptorSetLayoutHandle(id) => {
                    let layout = self.resources.descriptor_set_layouts.get_mut(id).unwrap();
                    layout.ref_count -= 1;
                    if layout.ref_count == 0 {
                        self.resources.descriptor_set_layouts.remove(id);
                    }
                }
                LifecycleEvent::CloneBufferHandle(id) => {
                    let buffer = self.resources.buffers.get_mut(id).unwrap();
                    buffer.ref_count += 1;
                }
                LifecycleEvent::CloneTextureHandle(id) => {
                    let texture = self.resources.textures.get_mut(id).unwrap();
                    texture.ref_count += 1;
                }
                LifecycleEvent::CloneSamplerHandle(id) => {
                    let sampler = self.resources.samplers.get_mut(id).unwrap();
                    sampler.ref_count += 1;
                }
                LifecycleEvent::ClonePipelineHandle(id) => {
                    let pipeline = self.resources.pipelines.get_mut(id).unwrap();
                    pipeline.ref_count += 1;
                }
                LifecycleEvent::CloneDescriptorSetLayoutHandle(id) => {
                    let layout = self.resources.descriptor_set_layouts.get_mut(id).unwrap();
                    layout.ref_count += 1;
                }
                LifecycleEvent::CloneDescriptorSetHandle(id) => {
                    let set = self.resources.descriptor_sets.get_mut(id).unwrap();
                    set.ref_count += 1;
                }
            }
        }
    }
}

#[derive(Debug)]
struct Resources {
    buffers: Arena<BufferInner>,
    textures: Arena<TextureInner>,
    samplers: Arena<SamplerInner>,
    descriptor_set_layouts: Arena<DescriptorSetLayoutInner>,
    descriptor_sets: Arena<DescriptorSetInner>,
    pipelines: Arena<PipelineInner>,
    allocator: GeneralPurposeAllocator,
    descriptor_allocator: DescriptorSetAllocator,
    lifecycle_events: Arc<SegQueue<LifecycleEvent>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum ResourceId {
    Buffer(BufferId),
    Texture(TextureMip),
}

impl ResourceMap for Resources {
    type Id = ResourceId;

    fn access(&self, id: Self::Id) -> AccessFlags {
        match id {
            ResourceId::Buffer(id) => self.buffers.get(id).unwrap().access,
            ResourceId::Texture(tex) => {
                let texture = self.textures.get(tex.id).unwrap();
                texture.mips[tex.mip_level as usize]
            }
        }
    }

    fn set_access(&mut self, id: Self::Id, access: AccessFlags) {
        match id {
            ResourceId::Buffer(id) => self.buffers.get_mut(id).unwrap().access = access,
            ResourceId::Texture(tex) => {
                let texture = self.textures.get_mut(tex.id).unwrap();
                texture.mips[tex.mip_level as usize] = access;
            }
        }
    }
}

#[derive(Debug)]
pub struct CommandQueue<'a> {
    executor: &'a mut CommandExecutor,
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

    pub fn create_buffer(&mut self, descriptor: &BufferDescriptor) -> Buffer {
        let buffer = self.executor.resources.allocator.create_buffer(
            descriptor.size.try_into().unwrap(),
            descriptor.usage,
            UsageFlags::HOST_VISIBLE,
        );

        let id = self.executor.resources.buffers.insert(BufferInner {
            buffer,
            access: AccessFlags::empty(),
            ref_count: 1,
        });

        Buffer {
            id,
            size: descriptor.size,
            usage: descriptor.usage,
            flags: descriptor.flags,
            events: self.executor.resources.lifecycle_events.clone(),
        }
    }

    pub fn create_buffer_init(&mut self, descriptor: &BufferInitDescriptor<'_>) -> Buffer {
        let buffer = self.create_buffer(&BufferDescriptor {
            size: descriptor.contents.len() as u64,
            usage: descriptor.usage | BufferUsage::TRANSFER_DST,
            flags: descriptor.flags,
        });
        self.write_buffer(&buffer, descriptor.contents);
        buffer
    }

    pub fn write_buffer(&mut self, buffer: &Buffer, data: &[u8]) {
        assert!(
            buffer.usage.contains(BufferUsage::TRANSFER_DST),
            "Buffer cannot be written to: TRANSFER_DST not set",
        );

        assert!(
            buffer.size as usize >= data.len(),
            "Buffer size of {} too small to copy {} bytes",
            buffer.size,
            data.len(),
        );

        // If the buffer is host visible we can map and write
        // to it directly.
        if buffer.flags.contains(UsageFlags::HOST_VISIBLE) {
            // The destination buffer must be kept alive until
            // the memcpy is complete.
            self.executor
                .resources
                .buffers
                .get_mut(buffer.id)
                .unwrap()
                .ref_count += 1;

            self.executor.cmds.push(
                &self.executor.resources,
                Command::WriteBuffer(WriteBuffer {
                    buffer: buffer.id,
                    data: data.to_vec(),
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

            debug_assert!(buffer.size >= staging_buffer.size);

            // The staging buffer must be kept alive until
            // the memcpy is complete.
            self.executor
                .resources
                .buffers
                .get_mut(staging_buffer.id)
                .unwrap()
                .ref_count += 1;

            // Write the data into the staging buffer.
            self.executor.cmds.push(
                &self.executor.resources,
                Command::WriteBuffer(WriteBuffer {
                    buffer: staging_buffer.id,
                    data: data.to_vec(),
                }),
            );

            self.copy_buffer_to_buffer(&staging_buffer, buffer);
        }
    }

    #[track_caller]
    pub fn create_texture(&mut self, descriptor: &TextureDescriptor) -> Texture {
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

        let supported_usages = self
            .executor
            .adapter_props
            .formats
            .get(&descriptor.format)
            .copied()
            .unwrap_or(TextureUsage::empty());
        if supported_usages.contains(descriptor.usage) {}

        let texture = self
            .executor
            .resources
            .allocator
            .create_texture(&descriptor, UsageFlags::empty());

        let id = self.executor.resources.textures.insert(TextureInner {
            data: TextureData::Virtual(texture),
            ref_count: 1,
            mips: vec![AccessFlags::empty(); descriptor.mip_levels as usize],
        });

        Texture {
            id,
            size: descriptor.size,
            format: descriptor.format,
            usage: descriptor.usage,
            events: self.executor.resources.lifecycle_events.clone(),
            auto_destroy: true,
            mip_levels: descriptor.mip_levels,
        }
    }

    #[track_caller]
    pub(crate) fn import_texture(
        &mut self,
        texture: vulkan::Texture,
        access: AccessFlags,
        usage: TextureUsage,
    ) -> Texture {
        let size = texture.size();
        let format = texture.format();
        let mip_levels = texture.mip_levels();

        let id = self.executor.resources.textures.insert(TextureInner {
            data: TextureData::Physical(texture),
            ref_count: 1,
            mips: vec![access; mip_levels as usize],
        });

        Texture {
            id,
            size,
            format,
            usage,
            events: self.executor.resources.lifecycle_events.clone(),
            auto_destroy: false,
            mip_levels,
        }
    }

    pub(crate) fn remove_imported_texture(&mut self, texture: Texture) {
        for (_, descriptor_set) in self.executor.resources.descriptor_sets.iter() {
            assert!(
                !descriptor_set
                    .textures
                    .iter()
                    .any(|(_, view)| view.texture == texture.id),
                "Texture cannot be removed: it is used in a descriptor set",
            );
        }

        let tex = self
            .executor
            .resources
            .textures
            .get_mut(texture.id)
            .unwrap();
        tex.ref_count -= 1;
        if tex.ref_count != 0 {
            panic!("Texture is still in use");
        }

        self.executor.resources.textures.remove(texture.id);
        debug_assert!(!texture.auto_destroy);
    }

    #[track_caller]
    pub fn write_texture(
        &mut self,
        texture: TextureRegion<'_>,
        data: &[u8],
        layout: ImageDataLayout,
    ) {
        assert!(
            texture.texture.usage.contains(TextureUsage::TRANSFER_DST),
            "Texture cannot be written to: TRANSFER_DST usage not set",
        );

        assert_eq!(
            data.len(),
            layout.bytes_per_row as usize * layout.rows_per_image as usize
        );

        let staging_buffer = self.create_buffer_init(&BufferInitDescriptor {
            contents: data,
            usage: BufferUsage::TRANSFER_SRC,
            flags: UsageFlags::HOST_VISIBLE,
        });

        self.copy_buffer_to_texture(&staging_buffer, texture, layout);
    }

    pub fn copy_buffer_to_buffer(&mut self, src: &Buffer, dst: &Buffer) {
        assert!(
            src.usage.contains(BufferUsage::TRANSFER_SRC),
            "Buffer cannot be read from: TRANSFER_SRC usage not set",
        );
        assert!(
            dst.usage.contains(BufferUsage::TRANSFER_DST),
            "Buffer cannot be written to: TRANSFER_DST not set",
        );

        assert!(
            src.size >= dst.size,
            "invalid buffer copy: source buffer (size={}) is smaller than destination buffer (size={})",
            src.size,
            dst.size,
        );

        // We don't actually have to copy anything if the destination is
        // empty.
        let Some(count) = NonZeroU64::new(dst.size) else {
            return;
        };

        // The source and destination buffer must be kept alive
        // for this command.
        self.executor
            .resources
            .buffers
            .get_mut(src.id)
            .unwrap()
            .ref_count += 1;
        self.executor
            .resources
            .buffers
            .get_mut(dst.id)
            .unwrap()
            .ref_count += 1;

        self.executor.cmds.push(
            &self.executor.resources,
            Command::CopyBufferToBuffer(CopyBufferToBuffer {
                src: src.id,
                src_offset: 0,
                dst: dst.id,
                dst_offset: 0,
                count,
            }),
        );
    }

    pub fn copy_buffer_to_texture(
        &mut self,
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
        self.executor
            .resources
            .buffers
            .get_mut(src.id)
            .unwrap()
            .ref_count += 1;
        self.executor
            .resources
            .textures
            .get_mut(dst.texture.id)
            .unwrap()
            .ref_count += 1;

        self.executor.cmds.push(
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

    pub fn copy_texture_to_texture(&mut self, src: TextureRegion<'_>, dst: TextureRegion<'_>) {
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
        self.executor
            .resources
            .textures
            .get_mut(src.texture.id)
            .unwrap()
            .ref_count += 1;
        self.executor
            .resources
            .textures
            .get_mut(dst.texture.id)
            .unwrap()
            .ref_count += 1;

        self.executor.cmds.push(
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
    pub fn create_descriptor_set(
        &mut self,
        descriptor: &DescriptorSetDescriptor<'_>,
    ) -> DescriptorSet {
        let mut buffers = Vec::new();
        let mut samplers = Vec::new();
        let mut textures = Vec::new();
        let mut texture_arrays = Vec::new();
        for entry in descriptor.entries {
            match entry.resource {
                BindingResource::Buffer(buffer) => {
                    assert!(
                        buffer.usage.contains(BufferUsage::UNIFORM)
                            || buffer.usage.contains(BufferUsage::STORAGE),
                        "Buffer cannot be bound to descriptor set: UNIFORM and STORAGE not set",
                    );

                    self.executor
                        .resources
                        .buffers
                        .get_mut(buffer.id)
                        .unwrap()
                        .ref_count += 1;

                    buffers.push((entry.binding, buffer.id));
                }
                BindingResource::Sampler(sampler) => {
                    self.executor
                        .resources
                        .samplers
                        .get_mut(sampler.id)
                        .unwrap()
                        .ref_count += 1;

                    samplers.push((entry.binding, sampler.id));
                }
                BindingResource::Texture(view) => {
                    assert!(
                        view.texture.usage.contains(TextureUsage::TEXTURE_BINDING),
                        "Texture cannot be bound to descriptor set: TEXTURE_BINDING not set",
                    );

                    self.executor
                        .resources
                        .textures
                        .get_mut(view.texture.id)
                        .unwrap()
                        .ref_count += 1;

                    textures.push((
                        entry.binding,
                        RawTextureView {
                            texture: view.texture.id,
                            base_mip_level: view.base_mip_level,
                            mip_levels: view.mip_levels,
                        },
                    ));
                }
                BindingResource::TextureArray(views) => {
                    for view in views {
                        assert!(
                            view.texture.usage.contains(TextureUsage::TEXTURE_BINDING),
                            "Texture cannot be bound to descriptor set: TEXTURE_BINDING not set",
                        );
                    }

                    for view in views {
                        self.executor
                            .resources
                            .textures
                            .get_mut(view.texture.id)
                            .unwrap()
                            .ref_count += 1;
                    }

                    texture_arrays.push((
                        entry.binding,
                        views
                            .into_iter()
                            .map(|view| RawTextureView {
                                texture: view.texture.id,
                                base_mip_level: view.base_mip_level,
                                mip_levels: view.mip_levels,
                            })
                            .collect(),
                    ));
                }
            }
        }

        self.executor
            .resources
            .descriptor_set_layouts
            .get_mut(descriptor.layout.id)
            .unwrap()
            .ref_count += 1;

        let id = self
            .executor
            .resources
            .descriptor_sets
            .insert(DescriptorSetInner {
                buffers,
                samplers,
                textures,
                texture_arrays,
                descriptor_set: None,
                layout: descriptor.layout.id,
                physical_texture_views: Vec::new(),
                ref_count: 1,
            });

        DescriptorSet {
            id,
            events: self.executor.resources.lifecycle_events.clone(),
        }
    }

    pub fn create_descriptor_set_layout(
        &mut self,
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
                ref_count: 1,
            });

        DescriptorSetLayout {
            id,
            events: self.executor.resources.lifecycle_events.clone(),
        }
    }

    pub fn create_pipeline(&mut self, descriptor: &PipelineDescriptor<'_>) -> Pipeline {
        let mut descriptors = Vec::new();
        for layout in descriptor.descriptors {
            let layout = self
                .executor
                .resources
                .descriptor_set_layouts
                .get(layout.id)
                .unwrap();

            descriptors.push(&layout.inner);
        }

        let mut bindings = BindingMap::default();
        for stage in descriptor.stages {
            match stage {
                PipelineStage::Vertex(stage) => {
                    let instance = stage.shader.shader.instantiate(&backend::shader::Options {
                        bindings: HashMap::new(),
                        stage: ShaderStage::Vertex,
                        entry_point: &stage.entry,
                    });

                    for binding in instance.bindings() {
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
                    let instance = stage.shader.shader.instantiate(&backend::shader::Options {
                        bindings: HashMap::new(),
                        stage: ShaderStage::Fragment,
                        entry_point: &stage.entry,
                    });

                    for binding in instance.bindings() {
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
        let id = self.executor.resources.pipelines.insert(PipelineInner {
            inner,
            ref_count: 1,
            bindings,
        });

        Pipeline {
            id,
            events: self.executor.resources.lifecycle_events.clone(),
        }
    }

    pub fn create_sampler(&mut self, descriptor: &SamplerDescriptor) -> Sampler {
        let inner = self.executor.device.create_sampler(descriptor).unwrap();
        let id = self.executor.resources.samplers.insert(SamplerInner {
            inner,
            ref_count: 1,
        });

        Sampler {
            id,
            events: self.executor.resources.lifecycle_events.clone(),
        }
    }

    pub fn run_render_pass(&mut self, descriptor: &RenderPassDescriptor<'_>) -> RenderPass<'a, '_> {
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
                    .get_mut(a.target.texture.id)
                    .unwrap()
                    .ref_count += 1;

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
                .get_mut(attachment.texture.id)
                .unwrap()
                .ref_count += 1;

            DepthStencilAttachmentOwned {
                texture: attachment.texture.id,
                load_op: attachment.load_op,
                store_op: attachment.store_op,
            }
        });

        RenderPass {
            ctx: self,
            color_attachments,
            depth_stencil_attachment,
            cmds: Vec::new(),
            last_pipeline: None,
            last_index_buffer: None,
        }
    }

    /// Manually force a transition of the [`TextureRegion`] to the specified [`AccessFlags`].
    pub(crate) fn transition_texture(&mut self, texture: &TextureRegion<'_>, to: AccessFlags) {
        self.executor
            .resources
            .textures
            .get_mut(texture.texture.id)
            .unwrap()
            .ref_count += 1;

        self.executor.cmds.push(
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
    events: Arc<SegQueue<LifecycleEvent>>,
}

impl Clone for DescriptorSet {
    fn clone(&self) -> Self {
        self.events
            .push(LifecycleEvent::CloneDescriptorSetHandle(self.id));

        Self {
            id: self.id,
            events: self.events.clone(),
        }
    }
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        self.events
            .push(LifecycleEvent::DestroyDescriptorSetHandle(self.id));
    }
}

#[derive(Debug)]
pub struct Sampler {
    id: SamplerId,
    events: Arc<SegQueue<LifecycleEvent>>,
}

impl Clone for Sampler {
    fn clone(&self) -> Self {
        self.events
            .push(LifecycleEvent::CloneSamplerHandle(self.id));

        Self {
            id: self.id,
            events: self.events.clone(),
        }
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        self.events
            .push(LifecycleEvent::DestroySamplerHandle(self.id));
    }
}

#[derive(Debug)]
struct SamplerInner {
    inner: vulkan::Sampler,
    ref_count: usize,
}

#[derive(Debug)]
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
    events: Arc<SegQueue<LifecycleEvent>>,
}

impl Clone for DescriptorSetLayout {
    fn clone(&self) -> Self {
        self.events
            .push(LifecycleEvent::CloneDescriptorSetLayoutHandle(self.id));

        Self {
            id: self.id,
            events: self.events.clone(),
        }
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        self.events
            .push(LifecycleEvent::DestroyDescriptorSetLayoutHandle(self.id));
    }
}

#[derive(Debug)]
struct DescriptorSetLayoutInner {
    inner: vulkan::DescriptorSetLayout,
    ref_count: usize,
}

#[derive(Debug)]
pub struct Pipeline {
    id: PipelineId,
    events: Arc<SegQueue<LifecycleEvent>>,
}

impl Clone for Pipeline {
    fn clone(&self) -> Self {
        self.events
            .push(LifecycleEvent::ClonePipelineHandle(self.id));

        Self {
            id: self.id,
            events: self.events.clone(),
        }
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        self.events
            .push(LifecycleEvent::DestroyPipelineHandle(self.id));
    }
}

#[derive(Debug)]
struct PipelineInner {
    inner: vulkan::Pipeline,
    ref_count: usize,
    bindings: BindingMap,
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
    events: Arc<SegQueue<LifecycleEvent>>,
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        self.events.push(LifecycleEvent::CloneBufferHandle(self.id));

        Self {
            id: self.id,
            size: self.size,
            usage: self.usage,
            flags: self.flags,
            events: self.events.clone(),
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.events
            .push(LifecycleEvent::DestroyBufferHandle(self.id));
    }
}

#[derive(Debug)]
pub struct BufferInner {
    buffer: BufferAlloc,
    access: AccessFlags,
    ref_count: usize,
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
enum LifecycleEvent {
    DestroyBufferHandle(BufferId),
    DestroyTextureHandle(TextureId),
    DestroySamplerHandle(SamplerId),
    DestroyPipelineHandle(PipelineId),
    DestroyDescriptorSetHandle(DescriptorSetId),
    DestroyDescriptorSetLayoutHandle(DescriptorSetLayoutId),
    CloneBufferHandle(BufferId),
    CloneTextureHandle(TextureId),
    CloneSamplerHandle(SamplerId),
    CloneDescriptorSetHandle(DescriptorSetId),
    CloneDescriptorSetLayoutHandle(DescriptorSetLayoutId),
    ClonePipelineHandle(PipelineId),
}

#[derive(Debug)]
struct DescriptorSetInner {
    // (Binding, Resource)
    buffers: Vec<(u32, BufferId)>,
    samplers: Vec<(u32, SamplerId)>,
    textures: Vec<(u32, RawTextureView)>,
    texture_arrays: Vec<(u32, Vec<RawTextureView>)>,
    descriptor_set: Option<AllocatedDescriptorSet>,
    layout: DescriptorSetLayoutId,
    physical_texture_views: Vec<vulkan::TextureView<'static>>,
    ref_count: usize,
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
    events: Arc<SegQueue<LifecycleEvent>>,
    auto_destroy: bool,
    mip_levels: u32,
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
}

impl Clone for Texture {
    fn clone(&self) -> Self {
        if self.auto_destroy {
            self.events
                .push(LifecycleEvent::CloneTextureHandle(self.id));
        }

        Self {
            id: self.id,
            size: self.size,
            format: self.format,
            usage: self.usage,
            events: self.events.clone(),
            auto_destroy: self.auto_destroy,
            mip_levels: self.mip_levels,
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if self.auto_destroy {
            self.events
                .push(LifecycleEvent::DestroyTextureHandle(self.id));
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

#[derive(Debug)]
pub struct TextureInner {
    data: TextureData,
    ref_count: usize,
    /// Access flags of each mip level.
    mips: Vec<AccessFlags>,
}

#[derive(Debug)]
enum TextureData {
    Physical(vulkan::Texture),
    Virtual(TextureAlloc),
}

impl TextureData {
    fn texture(&self) -> &vulkan::Texture {
        match self {
            Self::Physical(data) => data,
            Self::Virtual(data) => data.texture(),
        }
    }
}

pub struct RenderPass<'a, 'b> {
    ctx: &'b mut CommandQueue<'a>,
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
            .get_mut(pipeline.id)
            .unwrap()
            .ref_count += 1;

        self.cmds.push(DrawCmd::SetPipeline(pipeline.id));
        self.last_pipeline = Some(pipeline.id);
    }

    pub fn set_descriptor_set(&mut self, index: u32, descriptor_set: &'b DescriptorSet) {
        assert!(self.last_pipeline.is_some(), "Pipeline is not set");

        let set = self
            .ctx
            .executor
            .resources
            .descriptor_sets
            .get_mut(descriptor_set.id)
            .unwrap();
        set.ref_count += 1;

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
            .get_mut(buffer.id)
            .unwrap()
            .ref_count += 1;

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
        assert!(
            buffer.buffer.size() >= min_size.into(),
            "index buffer of size {} is too small for indices={:?} format={:?}",
            buffer.buffer.size(),
            indices,
            format,
        );

        self.cmds
            .push(DrawCmd::Draw(DrawCall::DrawIndexed(DrawIndexed {
                indices,
                vertex_offset,
                instances,
            })));
    }
}

impl<'a, 'b> Drop for RenderPass<'a, 'b> {
    fn drop(&mut self) {
        self.ctx.executor.cmds.push(
            &self.ctx.executor.resources,
            Command::RenderPass(RenderPassCmd {
                color_attachments: self.color_attachments.clone(),
                depth_stencil_attachment: self.depth_stencil_attachment.clone(),
                cmds: core::mem::take(&mut self.cmds),
            }),
        );
    }
}

pub struct RenderPassDescriptor<'a> {
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
