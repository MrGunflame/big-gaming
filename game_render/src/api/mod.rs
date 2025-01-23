//! Rendering API

mod executor;
mod scheduler;

use std::collections::{HashMap, VecDeque};
use std::mem::ManuallyDrop;
use std::num::NonZeroU64;
use std::ops::Range;
use std::sync::{mpsc, Arc};

use executor::TemporaryResources;
use game_common::collections::arena::{Arena, Key};
use game_common::components::Color;
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::Mutex;
use scheduler::{Node, Resource, ResourceMap};

use crate::backend::allocator::{BufferAlloc, GeneralPurposeAllocator, TextureAlloc, UsageFlags};
use crate::backend::descriptors::{AllocatedDescriptorSet, DescriptorSetAllocator};
use crate::backend::vulkan::{self, CommandEncoder, Device, TextureView};
use crate::backend::{
    self, AccessFlags, AdapterMemoryProperties, BufferBarrier, BufferUsage, CopyBuffer,
    DepthStencilState, DescriptorType, Face, FrontFace, ImageDataLayout, IndexFormat, LoadOp,
    PipelineBarriers, PipelineStage, PrimitiveTopology, PushConstantRange, SamplerDescriptor,
    ShaderModule, ShaderSource, ShaderStages, StoreOp, TextureBarrier, TextureDescriptor,
    TextureFormat, TextureUsage, WriteDescriptorBinding, WriteDescriptorResource,
    WriteDescriptorResources,
};

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
    cmds: Vec<Command>,
    device: Device,
}

impl CommandExecutor {
    pub fn new(device: Device, memory_props: AdapterMemoryProperties) -> Self {
        let (lifecycle_events_tx, lifecycle_events_rx) = mpsc::channel();
        Self {
            resources: Resources {
                pipelines: Arena::new(),
                buffers: Arena::new(),
                textures: Arena::new(),
                descriptor_sets: Arena::new(),
                descriptor_set_layouts: Arena::new(),
                allocator: GeneralPurposeAllocator::new(device.clone(), memory_props),
                descriptor_allocator: DescriptorSetAllocator::new(device.clone()),
                samplers: Arena::new(),
                lifecycle_events_tx,
                lifecycle_events_rx: Mutex::new(lifecycle_events_rx),
            },
            cmds: Vec::new(),
            device,
        }
    }

    pub fn queue(&mut self) -> CommandQueue<'_> {
        CommandQueue { executor: self }
    }

    pub fn execute(&mut self, encoder: &mut CommandEncoder<'_>) -> TemporaryResources {
        let _span = trace_span!("CommandExecutor::execute").entered();

        let steps = scheduler::schedule(&mut self.resources, &self.cmds);
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

        let rx = self.resources.lifecycle_events_rx.lock();
        while let Ok(cmd) = rx.try_recv() {
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
                            .lifecycle_events_tx
                            .send(LifecycleEvent::DestroyBufferHandle(*id))
                            .unwrap();
                    }

                    for (_, id) in &set.textures {
                        self.resources
                            .lifecycle_events_tx
                            .send(LifecycleEvent::DestroyTextureHandle(*id))
                            .unwrap();
                    }

                    for (_, textures) in &set.texture_arrays {
                        for id in textures {
                            self.resources
                                .lifecycle_events_tx
                                .send(LifecycleEvent::DestroyTextureHandle(*id))
                                .unwrap();
                        }
                    }

                    self.resources
                        .lifecycle_events_tx
                        .send(LifecycleEvent::DestroyDescriptorSetLayoutHandle(set.layout))
                        .unwrap();

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
    lifecycle_events_tx: mpsc::Sender<LifecycleEvent>,
    lifecycle_events_rx: Mutex<mpsc::Receiver<LifecycleEvent>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum ResourceId {
    Buffer(BufferId),
    Texture(TextureId),
}

impl ResourceMap for Resources {
    type Id = ResourceId;

    fn access(&self, id: Self::Id) -> AccessFlags {
        match id {
            ResourceId::Buffer(id) => self.buffers.get(id).unwrap().access,
            ResourceId::Texture(id) => self.textures.get(id).unwrap().access,
        }
    }

    fn set_access(&mut self, id: Self::Id, access: AccessFlags) {
        match id {
            ResourceId::Buffer(id) => self.buffers.get_mut(id).unwrap().access = access,
            ResourceId::Texture(id) => self.textures.get_mut(id).unwrap().access = access,
        }
    }
}

#[derive(Debug)]
pub struct CommandQueue<'a> {
    executor: &'a mut CommandExecutor,
}

impl<'a> CommandQueue<'a> {
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
            events: self.executor.resources.lifecycle_events_tx.clone(),
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

            self.executor
                .cmds
                .push(Command::WriteBuffer(buffer.id, data.to_vec()));
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
            self.executor
                .cmds
                .push(Command::WriteBuffer(staging_buffer.id, data.to_vec()));

            self.copy_buffer_to_buffer(&staging_buffer, buffer);
        }
    }

    #[track_caller]
    pub fn create_texture(&mut self, descriptor: &TextureDescriptor) -> Texture {
        assert!(
            !descriptor.usage.is_empty(),
            "TextureUsage flags must not be empty",
        );

        let texture = self
            .executor
            .resources
            .allocator
            .create_texture(&descriptor, UsageFlags::HOST_VISIBLE);

        let id = self.executor.resources.textures.insert(TextureInner {
            data: TextureData::Virtual(texture),
            access: AccessFlags::empty(),
            ref_count: 1,
        });

        Texture {
            id,
            size: descriptor.size,
            format: descriptor.format,
            usage: descriptor.usage,
            events: self.executor.resources.lifecycle_events_tx.clone(),
            auto_destroy: true,
        }
    }

    #[track_caller]
    pub(crate) fn import_texture(
        &mut self,
        texture: &'static vulkan::Texture,
        access: AccessFlags,
        size: UVec2,
        format: TextureFormat,
        usage: TextureUsage,
    ) -> Texture {
        let id = self.executor.resources.textures.insert(TextureInner {
            data: TextureData::Physical(texture),
            access,
            ref_count: 1,
        });

        Texture {
            id,
            size,
            format,
            usage,
            events: self.executor.resources.lifecycle_events_tx.clone(),
            auto_destroy: false,
        }
    }

    pub(crate) fn remove_imported_texture(&mut self, texture: Texture) {
        for (_, descriptor_set) in self.executor.resources.descriptor_sets.iter() {
            assert!(
                !descriptor_set
                    .textures
                    .iter()
                    .any(|(_, id)| *id == texture.id),
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
    pub fn write_texture(&mut self, texture: &Texture, data: &[u8], layout: ImageDataLayout) {
        assert!(
            texture.usage.contains(TextureUsage::TRANSFER_DST),
            "Texture cannot be written to: TRANSFER_DST usage not set",
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

        self.executor
            .cmds
            .push(Command::CopyBufferToBuffer(CopyBufferToBuffer {
                src: src.id,
                src_offset: 0,
                dst: dst.id,
                dst_offset: 0,
                count,
            }));
    }

    pub fn copy_buffer_to_texture(&mut self, src: &Buffer, dst: &Texture, layout: ImageDataLayout) {
        assert!(
            src.usage.contains(BufferUsage::TRANSFER_SRC),
            "Buffer cannot be read from: TRANSFER_SRC usage not set",
        );
        assert!(
            dst.usage.contains(TextureUsage::TRANSFER_DST),
            "Texture cannot be written to: TRANSFER_DST not set",
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
            .get_mut(dst.id)
            .unwrap()
            .ref_count += 1;

        self.executor
            .cmds
            .push(Command::CopyBufferToTexture(CopyBufferToTexture {
                src: src.id,
                dst: dst.id,
                offset: 0,
                layout,
            }));
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
                BindingResource::Texture(texture) => {
                    assert!(
                        texture.usage.contains(TextureUsage::TEXTURE_BINDING),
                        "Texture cannot be bound to descriptor set: TEXTURE_BINDING not set",
                    );

                    self.executor
                        .resources
                        .textures
                        .get_mut(texture.id)
                        .unwrap()
                        .ref_count += 1;

                    textures.push((entry.binding, texture.id));
                }
                BindingResource::TextureArray(textures) => {
                    for texture in textures {
                        assert!(
                            texture.usage.contains(TextureUsage::TEXTURE_BINDING),
                            "Texture cannot be bound to descriptor set: TEXTURE_BINDING not set",
                        );
                    }

                    for texture in textures {
                        self.executor
                            .resources
                            .textures
                            .get_mut(texture.id)
                            .unwrap()
                            .ref_count += 1;
                    }

                    texture_arrays
                        .push((entry.binding, textures.into_iter().map(|t| t.id).collect()));
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
            events: self.executor.resources.lifecycle_events_tx.clone(),
        }
    }

    pub fn create_descriptor_set_layout(
        &mut self,
        descriptor: &DescriptorSetLayoutDescriptor<'_>,
    ) -> DescriptorSetLayout {
        let inner = self.executor.device.create_descriptor_layout(descriptor);
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
            events: self.executor.resources.lifecycle_events_tx.clone(),
        }
    }

    pub fn create_pipeline(&mut self, descriptor: &PipelineDescriptor<'_>) -> Pipeline {
        let descriptors = self
            .executor
            .resources
            .descriptor_set_layouts
            .iter()
            .filter_map(|(id, layout)| {
                descriptor
                    .descriptors
                    .iter()
                    .any(|d| d.id == id)
                    .then_some(&layout.inner)
            })
            .collect::<Vec<_>>();

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
            });
        let id = self.executor.resources.pipelines.insert(PipelineInner {
            inner,
            ref_count: 1,
        });

        Pipeline {
            id,
            events: self.executor.resources.lifecycle_events_tx.clone(),
        }
    }

    pub fn create_sampler(&mut self, descriptor: &SamplerDescriptor) -> Sampler {
        let inner = self.executor.device.create_sampler(descriptor);
        let id = self.executor.resources.samplers.insert(SamplerInner {
            inner,
            ref_count: 1,
        });

        Sampler {
            id,
            events: self.executor.resources.lifecycle_events_tx.clone(),
        }
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

                self.executor
                    .resources
                    .textures
                    .get_mut(a.texture.id)
                    .unwrap()
                    .ref_count += 1;

                ColorAttachmentOwned {
                    texture: a.texture.id,
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

    pub fn create_shader_module(&mut self, src: ShaderSource<'_>) -> ShaderModule {
        ShaderModule::new(&src, &self.executor.device)
    }
}

#[derive(Debug)]
pub struct DescriptorSet {
    id: DescriptorSetId,
    events: mpsc::Sender<LifecycleEvent>,
}

impl Clone for DescriptorSet {
    fn clone(&self) -> Self {
        self.events
            .send(LifecycleEvent::CloneDescriptorSetHandle(self.id))
            .ok();

        Self {
            id: self.id,
            events: self.events.clone(),
        }
    }
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        self.events
            .send(LifecycleEvent::DestroyDescriptorSetHandle(self.id))
            .ok();
    }
}

#[derive(Debug)]
pub struct Sampler {
    id: SamplerId,
    events: mpsc::Sender<LifecycleEvent>,
}

impl Clone for Sampler {
    fn clone(&self) -> Self {
        self.events
            .send(LifecycleEvent::CloneSamplerHandle(self.id))
            .ok();

        Self {
            id: self.id,
            events: self.events.clone(),
        }
    }
}

#[derive(Debug)]
struct SamplerInner {
    inner: vulkan::Sampler,
    ref_count: usize,
}

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
    events: mpsc::Sender<LifecycleEvent>,
}

impl Clone for DescriptorSetLayout {
    fn clone(&self) -> Self {
        self.events
            .send(LifecycleEvent::CloneDescriptorSetLayoutHandle(self.id))
            .ok();

        Self {
            id: self.id,
            events: self.events.clone(),
        }
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        self.events
            .send(LifecycleEvent::DestroyDescriptorSetLayoutHandle(self.id))
            .ok();
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
    events: mpsc::Sender<LifecycleEvent>,
}

impl Clone for Pipeline {
    fn clone(&self) -> Self {
        self.events
            .send(LifecycleEvent::ClonePipelineHandle(self.id))
            .ok();

        Self {
            id: self.id,
            events: self.events.clone(),
        }
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        self.events
            .send(LifecycleEvent::DestroyPipelineHandle(self.id))
            .ok();
    }
}

#[derive(Debug)]
struct PipelineInner {
    inner: vulkan::Pipeline,
    ref_count: usize,
}

#[derive(Debug)]
pub struct Buffer {
    id: BufferId,
    size: u64,
    usage: BufferUsage,
    flags: UsageFlags,
    events: mpsc::Sender<LifecycleEvent>,
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        self.events
            .send(LifecycleEvent::CloneBufferHandle(self.id))
            .ok();

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
            .send(LifecycleEvent::DestroyBufferHandle(self.id))
            .ok();
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
    Texture(&'a Texture),
    TextureArray(&'a [&'a Texture]),
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
enum Command {
    WriteBuffer(BufferId, Vec<u8>),
    CopyBufferToBuffer(CopyBufferToBuffer),
    CopyBufferToTexture(CopyBufferToTexture),
    RenderPass(RenderPassCmd),
}

impl Node<Resources> for Command {
    type ResourceId = ResourceId;

    fn resources(&self, resources: &Resources) -> Vec<Resource<Self::ResourceId>> {
        match self {
            Self::WriteBuffer(id, _) => {
                vec![Resource {
                    id: ResourceId::Buffer(*id),
                    access: AccessFlags::TRANSFER_WRITE,
                }]
            }
            Self::CopyBufferToBuffer(cmd) => {
                vec![
                    Resource {
                        id: ResourceId::Buffer(cmd.src),
                        access: AccessFlags::TRANSFER_READ,
                    },
                    Resource {
                        id: ResourceId::Buffer(cmd.dst),
                        access: AccessFlags::TRANSFER_WRITE,
                    },
                ]
            }
            Self::CopyBufferToTexture(cmd) => {
                vec![
                    Resource {
                        id: ResourceId::Buffer(cmd.src),
                        access: AccessFlags::TRANSFER_READ,
                    },
                    Resource {
                        id: ResourceId::Texture(cmd.dst),
                        access: AccessFlags::TRANSFER_WRITE,
                    },
                ]
            }
            Self::RenderPass(cmd) => {
                let mut accesses = HashMap::new();

                for cmd in &cmd.cmds {
                    match cmd {
                        DrawCmd::SetPipeline(_) => {}
                        DrawCmd::SetIndexBuffer(buffer, _) => {
                            *accesses.entry(ResourceId::Buffer(*buffer)).or_default() |=
                                AccessFlags::INDEX;
                        }
                        DrawCmd::SetDescriptorSet(_, id) => {
                            let descriptor_set = resources.descriptor_sets.get(*id).unwrap();
                            for (_, buffer) in &descriptor_set.buffers {
                                *accesses.entry(ResourceId::Buffer(*buffer)).or_default() |=
                                    AccessFlags::SHADER_READ;
                            }

                            for (_, texture) in &descriptor_set.textures {
                                *accesses.entry(ResourceId::Texture(*texture)).or_default() |=
                                    AccessFlags::SHADER_READ;
                            }

                            for (_, textures) in &descriptor_set.texture_arrays {
                                for texture in textures {
                                    *accesses.entry(ResourceId::Texture(*texture)).or_default() |=
                                        AccessFlags::SHADER_READ;
                                }
                            }
                        }
                        DrawCmd::SetPushConstants(_, _, _) => (),
                        DrawCmd::Draw(_) => (),
                    }
                }

                for attachment in &cmd.color_attachments {
                    *accesses
                        .entry(ResourceId::Texture(attachment.texture))
                        .or_default() |= AccessFlags::COLOR_ATTACHMENT_WRITE;
                }

                if let Some(attachment) = &cmd.depth_stencil_attachment {
                    *accesses
                        .entry(ResourceId::Texture(attachment.texture))
                        .or_default() |=
                        AccessFlags::DEPTH_ATTACHMENT_READ | AccessFlags::DEPTH_ATTACHMENT_WRITE;
                }

                accesses
                    .into_iter()
                    .map(|(id, access)| Resource { id, access })
                    .collect()
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct CopyBufferToBuffer {
    src: BufferId,
    src_offset: u64,
    dst: BufferId,
    dst_offset: u64,
    count: NonZeroU64,
}

#[derive(Clone, Debug)]
struct CopyBufferToTexture {
    src: BufferId,
    offset: u64,
    layout: ImageDataLayout,
    dst: TextureId,
}

#[derive(Debug)]
struct DescriptorSetInner {
    // (Binding, Resource)
    buffers: Vec<(u32, BufferId)>,
    samplers: Vec<(u32, SamplerId)>,
    textures: Vec<(u32, TextureId)>,
    texture_arrays: Vec<(u32, Vec<TextureId>)>,
    descriptor_set: Option<AllocatedDescriptorSet>,
    layout: DescriptorSetLayoutId,
    physical_texture_views: Vec<TextureView<'static>>,
    ref_count: usize,
}

#[derive(Clone, Debug)]
struct RenderPassCmd {
    color_attachments: Vec<ColorAttachmentOwned>,
    depth_stencil_attachment: Option<DepthStencilAttachmentOwned>,
    cmds: Vec<DrawCmd>,
}

#[derive(Debug)]
pub struct Texture {
    id: TextureId,
    size: UVec2,
    format: TextureFormat,
    usage: TextureUsage,
    events: mpsc::Sender<LifecycleEvent>,
    auto_destroy: bool,
}

impl Texture {
    pub fn size(&self) -> UVec2 {
        self.size
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }
}

impl Clone for Texture {
    fn clone(&self) -> Self {
        if self.auto_destroy {
            self.events
                .send(LifecycleEvent::CloneTextureHandle(self.id))
                .ok();
        }

        Self {
            id: self.id,
            size: self.size,
            format: self.format,
            usage: self.usage,
            events: self.events.clone(),
            auto_destroy: self.auto_destroy,
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if self.auto_destroy {
            self.events
                .send(LifecycleEvent::DestroyTextureHandle(self.id))
                .ok();
        }
    }
}

#[derive(Debug)]
pub struct TextureInner {
    data: TextureData,
    access: AccessFlags,
    ref_count: usize,
}

#[derive(Debug)]
enum TextureData {
    Physical(&'static vulkan::Texture),
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
        self.ctx
            .executor
            .cmds
            .push(Command::RenderPass(RenderPassCmd {
                color_attachments: self.color_attachments.clone(),
                depth_stencil_attachment: self.depth_stencil_attachment.clone(),
                cmds: core::mem::take(&mut self.cmds),
            }));
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
    texture: TextureId,
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
