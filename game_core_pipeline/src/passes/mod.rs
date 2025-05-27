mod forward;
mod opaque_vertex;
mod post_process;
mod state;
mod update;

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::atomic::fence;

use bitflags::bitflags;
use bytemuck::{NoUninit, Pod, Zeroable};
use crossbeam::channel::Receiver;
use forward::ForwardPass;
use game_common::collections::arena::Key;
use game_common::components::Transform;
use game_render::api::{
    CommandQueue, DescriptorSetLayout, TextureRegion, TextureView, TextureViewDescriptor,
};
use game_render::backend::{
    BufferUsage, DescriptorBinding, DescriptorSetDescriptor, DescriptorType, ImageDataLayout,
    ShaderStages, TextureDescriptor, TextureFormat, TextureUsage,
};
use game_render::buffer::GpuBuffer;
use game_render::buffer::slab::{CompactSlabBuffer, SlabBuffer, SlabIndex};
use game_render::graph::{NodeLabel, RenderGraph, SlotFlags, SlotKind, SlotLabel};
use game_render::mesh::Mesh;
use glam::UVec2;
use opaque_vertex::state::VertexMeshState;
use parking_lot::Mutex;
use post_process::PostProcessPass;
use state::mesh::MeshState;
use update::{TransformUniform, UpdatePass};

use crate::camera::Camera;
use crate::entities::{CameraId, Event, ImageId, LightId, MaterialId, MeshId, ObjectId, SceneId};
use crate::lights::{DirectionalLightUniform, PointLightUniform, SpotLightUniform};

const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

const UPDATE_PASS: NodeLabel = NodeLabel::new("UPDATE_PASS");
const FORWARD_PASS: NodeLabel = NodeLabel::new("FORWARD_PASS");
const POST_PROCESS_PASS: NodeLabel = NodeLabel::new("POST_PROCESS_PASS");
pub const FINAL_RENDER_PASS: NodeLabel = NodeLabel::new("FINAL_RENDER_PASS");

const HDR_TEXTURE: SlotLabel = SlotLabel::new("HDR_TEXTURE");

pub(crate) fn init(graph: &mut RenderGraph, queue: &CommandQueue<'_>, events: Receiver<Event>) {
    let state = Arc::new(Mutex::new(State::new(queue)));

    let update_pass = UpdatePass::new(queue, state.clone(), events);
    graph.add_node(UPDATE_PASS, update_pass);

    if queue.features().mesh_shader {
        let forward_pass = ForwardPass::new(queue, state.clone(), HDR_TEXTURE);
        graph.add_node(FORWARD_PASS, forward_pass);
        graph.add_node_dependency(FORWARD_PASS, UPDATE_PASS);

        graph.add_slot_dependency(
            FORWARD_PASS,
            SlotLabel::SURFACE,
            SlotKind::Texture,
            SlotFlags::READ,
        );
        graph.add_slot_dependency(
            FORWARD_PASS,
            HDR_TEXTURE,
            SlotKind::Texture,
            SlotFlags::WRITE,
        );
    } else {
        opaque_vertex::init(graph, queue, state);
    }

    let post_process_pass = PostProcessPass::new(queue, HDR_TEXTURE, SlotLabel::SURFACE);
    graph.add_node(POST_PROCESS_PASS, post_process_pass);
    graph.add_slot_dependency(
        POST_PROCESS_PASS,
        HDR_TEXTURE,
        SlotKind::Texture,
        SlotFlags::READ,
    );
    graph.add_slot_dependency(
        POST_PROCESS_PASS,
        SlotLabel::SURFACE,
        SlotKind::Texture,
        SlotFlags::READ | SlotFlags::WRITE,
    );
}

#[derive(Debug)]
struct State {
    default_textures: DefaultTextures,

    mesh_descriptor_layout: DescriptorSetLayout,

    mesh: MeshStateImpl,

    meshes: HashMap<MeshId, MeshKey>,
    images: HashMap<ImageId, TextureSlabIndex>,
    materials: HashMap<MaterialId, SlabIndex>,

    scenes: HashMap<SceneId, SceneData>,

    textures: TextureSlab,
    material_slab: SlabBuffer<RawMaterialData>,
}

impl State {
    fn new(queue: &CommandQueue<'_>) -> Self {
        let mesh_descriptor_layout = queue.create_descriptor_set_layout(&DescriptorSetDescriptor {
            bindings: &[
                // POSITIONS
                DescriptorBinding {
                    binding: 0,
                    visibility: ShaderStages::MESH,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                // NORMALS
                DescriptorBinding {
                    binding: 1,
                    visibility: ShaderStages::MESH,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                // TANGENTS
                DescriptorBinding {
                    binding: 2,
                    visibility: ShaderStages::MESH,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                // UVS
                DescriptorBinding {
                    binding: 3,
                    visibility: ShaderStages::MESH,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                // VERTEX INDICES
                DescriptorBinding {
                    binding: 4,
                    visibility: ShaderStages::MESH,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                // TRIANGLE INDICES
                DescriptorBinding {
                    binding: 5,
                    visibility: ShaderStages::MESH,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                // MESHLETS
                DescriptorBinding {
                    binding: 6,
                    visibility: ShaderStages::MESH,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                // INSTANCES
                DescriptorBinding {
                    binding: 7,
                    visibility: ShaderStages::TASK | ShaderStages::MESH,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
            ],
        });

        let placeholder_texture = queue.create_texture(&TextureDescriptor {
            size: UVec2::ONE,
            mip_levels: 1,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsage::TEXTURE_BINDING,
        });

        let mut textures =
            TextureSlab::new(placeholder_texture.create_view(&TextureViewDescriptor::default()));

        let default_textures = DefaultTextures::new(queue, &mut textures);

        Self {
            default_textures,
            mesh_descriptor_layout,
            meshes: HashMap::new(),
            materials: HashMap::new(),
            scenes: HashMap::new(),
            images: HashMap::new(),
            textures,
            material_slab: SlabBuffer::new(BufferUsage::STORAGE),
            mesh: MeshStateImpl::new(queue),
        }
    }
}

#[derive(Debug)]
struct DefaultTextures {
    base_color: TextureSlabIndex,
    normal: TextureSlabIndex,
    metallic_roughness: TextureSlabIndex,
    specular_glossiness: TextureSlabIndex,
}

impl DefaultTextures {
    fn new(queue: &CommandQueue<'_>, textures: &mut TextureSlab) -> Self {
        let [base_color, normal, metallic_roughness, specular_glossiness] = [
            // Base color: white
            (TextureFormat::Rgba8UnormSrgb, [255, 255, 255, 255]),
            // Normal: B channel facing towards local Z.
            (TextureFormat::Rgba8Unorm, [127, 127, 255, 255]),
            // Roughness (channel G): Factor 1.0.
            // Meallic (channel B): Factor 1.0.
            // Other channels are ignored.
            (TextureFormat::Rgba8UnormSrgb, [0, 255, 255, 0]),
            // Specular (channel R): Factor 1.0.
            // Glossiness (channel G): 0.0, results in roughness of 1.0.
            // Other channels are ignored.
            (TextureFormat::Rgba8UnormSrgb, [255, 0, 0, 0]),
        ]
        .map(|(format, data)| {
            let texture = queue.create_texture(&TextureDescriptor {
                size: UVec2::ONE,
                mip_levels: 1,
                format,
                usage: TextureUsage::TRANSFER_DST | TextureUsage::TEXTURE_BINDING,
            });

            queue.write_texture(
                TextureRegion {
                    texture: &texture,
                    mip_level: 0,
                },
                &data,
                ImageDataLayout {
                    bytes_per_row: 4,
                    rows_per_image: 1,
                },
            );

            textures.insert(texture.create_view(&TextureViewDescriptor::default()))
        });

        Self {
            base_color,
            normal,
            metallic_roughness,
            specular_glossiness,
        }
    }
}

#[derive(Debug)]
struct SceneData {
    cameras: HashMap<CameraId, Camera>,
    objects: HashMap<ObjectId, ObjectState>,
    directional_lights: HashMap<LightId, SlabIndex>,
    point_lights: HashMap<LightId, SlabIndex>,
    spot_lights: HashMap<LightId, SlabIndex>,

    directional_lights_buffer: CompactSlabBuffer<DirectionalLightUniform>,
    point_lights_buffer: CompactSlabBuffer<PointLightUniform>,
    spot_lights_buffer: CompactSlabBuffer<SpotLightUniform>,
}

impl SceneData {
    fn new() -> Self {
        Self {
            cameras: HashMap::new(),
            objects: HashMap::new(),
            directional_lights: HashMap::new(),
            point_lights: HashMap::new(),
            spot_lights: HashMap::new(),
            directional_lights_buffer: CompactSlabBuffer::new(BufferUsage::STORAGE),
            point_lights_buffer: CompactSlabBuffer::new(BufferUsage::STORAGE),
            spot_lights_buffer: CompactSlabBuffer::new(BufferUsage::STORAGE),
        }
    }
}

#[derive(Debug)]
struct ObjectState {
    id: InstanceKey,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct TextureSlabIndex(u32);

#[derive(Debug)]
pub struct TextureSlab {
    default: TextureView,
    entries: Vec<Option<TextureView>>,
    free_list: Vec<u32>,
}

impl TextureSlab {
    pub fn new(default: TextureView) -> Self {
        Self {
            default,
            entries: Vec::new(),
            free_list: Vec::new(),
        }
    }

    pub fn insert(&mut self, texture: TextureView) -> TextureSlabIndex {
        let index = match self.free_list.pop() {
            Some(index) => index as usize,
            None => {
                let index = self.entries.len();
                self.entries.resize(index + 1, None);
                index
            }
        };

        let slot = &mut self.entries[index];
        debug_assert!(slot.is_none());
        *slot = Some(texture);

        TextureSlabIndex(index as u32)
    }

    pub fn remove(&mut self, index: TextureSlabIndex) {
        let entry = &mut self.entries[index.0 as usize];
        debug_assert!(entry.is_some());
        *entry = None;

        self.free_list.push(index.0);
        // TODO: We may want to compact the entries when the last
        // entry is removed.
        // This will require to iteratively pop entries from the
        // back of the entries Vec AND remove the index from
        // the free list.
    }

    pub fn views(&self) -> Vec<&TextureView> {
        self.entries
            .iter()
            .map(|view| view.as_ref().unwrap_or(&self.default))
            .collect()
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct RawMaterialData {
    flags: MaterialFlags,
    _pad0: [u32; 3],
    base_color: [f32; 4],
    roughness: f32,
    metallic: f32,
    reflectance: f32,
    specular_strength: f32,
    specular_color: [f32; 4],
    base_color_texture_index: TextureSlabIndex,
    normal_texture_index: TextureSlabIndex,
    metallic_roughness_texture_index: TextureSlabIndex,
    specular_glossiness_texture_index: TextureSlabIndex,
}

impl GpuBuffer for RawMaterialData {
    const SIZE: usize = size_of::<Self>();
    const ALIGN: usize = 16;
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    #[repr(transparent)]
    struct MaterialFlags: u32 {
        const UNLIT = 1 << 0;
        const FLIP_NORMAL_Y = 1 << 1;
        const METALLIC_FROM_SPECULAR = 1 << 2;
        const ROUGHNESS_FROM_GLOSSINESS = 1 << 3;
        const NORMAL_ENCODING_TWO_CHANNEL = 1 << 4;
    }
}

#[derive(Copy, Clone, Debug, NoUninit)]
#[repr(C)]
struct RawObjectData {
    pub transform: TransformUniform,
    pub meshlet_offset: u32,
    pub material: SlabIndex,
    _pad0: [u32; 2],
}

#[derive(Debug)]
enum MeshStateImpl {
    Vertex(VertexMeshState),
    Mesh(MeshState),
}

impl MeshStateImpl {
    fn new(queue: &CommandQueue<'_>) -> Self {
        if queue.features().mesh_shader {
            Self::Mesh(MeshState::new(queue))
        } else {
            Self::Vertex(VertexMeshState::new(queue))
        }
    }

    fn create_mesh(&mut self, queue: &CommandQueue<'_>, mesh: &Mesh) -> MeshKey {
        match self {
            Self::Mesh(state) => state.create_mesh(queue, mesh),
            Self::Vertex(state) => state.create_mesh(queue, mesh),
        }
    }

    fn remove_mesh(&mut self, key: MeshKey) {
        match self {
            Self::Mesh(state) => state.remove_mesh(key),
            Self::Vertex(state) => state.remove_mesh(key),
        }
    }

    fn create_instance(
        &mut self,
        transform: Transform,
        mesh: MeshKey,
        material: SlabIndex,
    ) -> InstanceKey {
        match self {
            Self::Mesh(state) => state.create_instance(transform, mesh, material),
            Self::Vertex(state) => state.create_instance(transform, mesh, material),
        }
    }

    fn remove_instance(&mut self, key: InstanceKey) {
        match self {
            Self::Mesh(state) => state.remove_instance(key),
            Self::Vertex(state) => state.remove_instance(key),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshKey(pub Key);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InstanceKey(SlabIndex);
