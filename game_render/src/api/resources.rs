use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crossbeam_queue::SegQueue;
use game_common::cell::UnsafeRefCell;
use sharded_slab::Slab;

use crate::backend::allocator::{BufferAlloc, GeneralPurposeAllocator, TextureAlloc};
use crate::backend::descriptors::{AllocatedDescriptorSet, DescriptorSetAllocator};
use crate::backend::{vulkan, AccessFlags};

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
    pub allocator: GeneralPurposeAllocator,
    pub descriptor_allocator: DescriptorSetAllocator,
    pub deletion_queue: SegQueue<DeletionEvent>,
}

impl<'a> ResourceMap for &'a Resources {
    type Id = ResourceId;

    fn access(&self, id: Self::Id) -> AccessFlags {
        match id {
            ResourceId::Buffer(buffer) => {
                let buffer = self.buffers.get(buffer).unwrap();
                let access = unsafe { *buffer.access.borrow() };
                access
            }
            ResourceId::Texture(texture) => {
                let tex = self.textures.get(texture.id).unwrap();
                let mip = &tex.mip_access[texture.mip_level as usize];
                let access = unsafe { *mip.borrow() };
                access
            }
        }
    }

    fn set_access(&mut self, id: Self::Id, access: AccessFlags) {
        match id {
            ResourceId::Buffer(buffer) => {
                let buffer = self.buffers.get(buffer).unwrap();
                unsafe {
                    *buffer.access.borrow_mut() = access;
                }
            }
            ResourceId::Texture(texture) => {
                let tex = self.textures.get(texture.id).unwrap();
                let mip = &tex.mip_access[texture.mip_level as usize];
                unsafe {
                    *mip.borrow_mut() = access;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct BufferInner {
    pub buffer: UnsafeRefCell<BufferAlloc>,
    pub access: UnsafeRefCell<AccessFlags>,
    pub ref_count: RefCount,
}

#[derive(Debug)]
pub struct TextureInner {
    pub texture: TextureData,
    pub ref_count: RefCount,
    pub mip_access: Vec<UnsafeRefCell<AccessFlags>>,
}

impl TextureInner {
    pub fn texture(&self) -> &vulkan::Texture {
        match &self.texture {
            TextureData::Physical(tex) => tex,
            TextureData::Virtual(tex) => tex.texture(),
        }
    }
}

#[derive(Debug)]
pub enum TextureData {
    Physical(vulkan::Texture),
    Virtual(TextureAlloc),
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
}

#[derive(Debug)]
pub struct DescriptorSetInner {
    // (Binding, Resource)
    pub buffers: Vec<(u32, BufferId)>,
    pub samplers: Vec<(u32, SamplerId)>,
    pub textures: Vec<(u32, RawTextureView)>,
    pub texture_arrays: Vec<(u32, Vec<RawTextureView>)>,
    pub descriptor_set: UnsafeRefCell<Option<AllocatedDescriptorSet>>,
    pub layout: DescriptorSetLayoutId,
    pub physical_texture_views: UnsafeRefCell<Vec<vulkan::TextureView<'static>>>,
    pub ref_count: RefCount,
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
