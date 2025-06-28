use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crossbeam_queue::SegQueue;
use game_common::cell::UnsafeRefCell;
use game_common::collections::vec_map::VecMap;
use parking_lot::Mutex;
use sharded_slab::Slab;

use crate::backend::descriptors::DescriptorSetAllocator;
use crate::backend::{vulkan, AccessFlags, DescriptorBinding};

use super::{BindingMap, DeletionEvent, RawTextureView, ResourceId, ResourceMap};

pub type BufferId = usize;
pub type TextureId = usize;
pub type DescriptorSetLayoutId = usize;
pub type DescriptorSetId = usize;
pub type SamplerId = usize;
pub type PipelineId = usize;

#[derive(Debug)]
pub struct Resources {
    pub buffers: Slab<BufferInner>,
    pub textures: Slab<TextureInner>,
    pub samplers: Slab<SamplerInner>,
    pub descriptor_set_layouts: Slab<DescriptorSetLayoutInner>,
    pub descriptor_sets: Slab<DescriptorSetInner>,
    pub pipelines: Slab<PipelineInner>,
    pub descriptor_allocator: DescriptorSetAllocator,
    pub deletion_queue: SegQueue<DeletionEvent>,
    /// Staging memory used to `WriteBuffer` commands.
    pub staging_memory: Mutex<Vec<u8>>,
}

impl<'a> ResourceMap for &'a Resources {
    type Id = ResourceId;

    fn access(&self, id: Self::Id) -> AccessFlags {
        match id {
            ResourceId::Buffer(buffer) => unsafe {
                let buffer = self.buffers.get(buffer).unwrap_unchecked();
                let access = *buffer.access.borrow();
                access
            },
            ResourceId::Texture(texture) => unsafe {
                let tex = self.textures.get(texture.id).unwrap_unchecked();
                let mip = tex.mip_access.get_unchecked(texture.mip_level as usize);
                let access = *mip.borrow();
                access
            },
        }
    }

    fn set_access(&mut self, id: Self::Id, access: AccessFlags) {
        match id {
            ResourceId::Buffer(buffer) => unsafe {
                let buffer = self.buffers.get(buffer).unwrap_unchecked();
                *buffer.access.borrow_mut() = access;
            },
            ResourceId::Texture(texture) => unsafe {
                let tex = self.textures.get(texture.id).unwrap_unchecked();
                let mip = tex.mip_access.get_unchecked(texture.mip_level as usize);
                *mip.borrow_mut() = access;
            },
        }
    }
}

#[derive(Debug)]
pub struct BufferInner {
    pub access: UnsafeRefCell<AccessFlags>,
    pub ref_count: RefCount,
}

#[derive(Debug)]
pub struct TextureInner {
    pub ref_count: RefCount,
    pub mip_access: Vec<UnsafeRefCell<AccessFlags>>,
}

#[derive(Debug)]
pub struct SamplerInner {
    pub inner: vulkan::Sampler,
    pub ref_count: RefCount,
}

#[derive(Debug)]
pub struct DescriptorSetLayoutInner {
    pub inner: vulkan::DescriptorSetLayout,
    pub ref_count: RefCount,
    pub bindings: VecMap<u32, DescriptorBinding>,
}

#[derive(Debug)]
pub struct DescriptorSetInner {
    pub bindings: VecMap<u32, DescriptorSetResource>,
    pub num_buffers: u32,
    pub num_samplers: u32,
    pub num_textures: u32,
    pub num_texture_arrays: u32,
    pub layout: DescriptorSetLayoutId,
    pub ref_count: RefCount,
}

/// A resource that can be bound to a descriptor set.
#[derive(Debug)]
pub enum DescriptorSetResource {
    UniformBuffer(BufferId),
    StorageBuffer(BufferId),
    Sampler(SamplerId),
    Texture(RawTextureView),
    TextureArray(Vec<RawTextureView>),
}

#[derive(Debug)]
pub struct PipelineInner {
    pub inner: Arc<vulkan::Pipeline>,
    pub bindings: BindingMap,
    pub ref_count: RefCount,
}

#[derive(Debug)]
pub struct RefCount(AtomicUsize);

impl RefCount {
    pub const fn new() -> Self {
        RefCount(AtomicUsize::new(1))
    }
}

impl RefCount {
    pub fn increment(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrements the reference counter and returns whether the refcount was the last one.
    #[must_use]
    pub fn decrement(&self) -> bool {
        self.decrement_many(1)
    }

    /// Decrements the reference counter by `count` at once.
    #[must_use]
    pub fn decrement_many(&self, count: usize) -> bool {
        if self.0.fetch_sub(count, Ordering::Release) != count {
            return false;
        }

        self.0.load(Ordering::Acquire);
        true
    }
}
