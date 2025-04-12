mod forward;
mod post_process;
mod update;

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use crossbeam::channel::Receiver;
use forward::ForwardPass;
use game_render::api::{
    Buffer, BufferInitDescriptor, CommandQueue, DescriptorSet, DescriptorSetLayout, Texture,
    TextureRegion,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    BufferUsage, DescriptorBinding, DescriptorSetDescriptor, DescriptorType, ImageDataLayout,
    ShaderStages, TextureDescriptor, TextureFormat, TextureUsage,
};
use game_render::buffer::{DynamicBuffer, IndexBuffer};
use game_render::graph::{NodeLabel, RenderGraph, SlotFlags, SlotKind, SlotLabel};
use glam::UVec2;
use parking_lot::Mutex;
use post_process::PostProcessPass;
use update::UpdatePass;

use crate::camera::Camera;
use crate::entities::{CameraId, Event, ImageId, LightId, MaterialId, MeshId, ObjectId, SceneId};
use crate::lights::{
    DirectionalLight, DirectionalLightUniform, PointLight, PointLightUniform, SpotLight,
    SpotLightUniform,
};

const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

const UPDATE_PASS: NodeLabel = NodeLabel::new("UPDATE_PASS");
const FORWARD_PASS: NodeLabel = NodeLabel::new("FORWARD_PASS");
const POST_PROCESS_PASS: NodeLabel = NodeLabel::new("POST_PROCESS_PASS");

const HDR_TEXTURE: SlotLabel = SlotLabel::new("HDR_TEXTURE");

pub(crate) fn init(graph: &mut RenderGraph, queue: &mut CommandQueue<'_>, events: Receiver<Event>) {
    let state = Arc::new(Mutex::new(State::new(queue)));

    let update_pass = UpdatePass::new(queue, state.clone(), events);
    graph.add_node(UPDATE_PASS, update_pass);

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
    material_descriptor_layout: DescriptorSetLayout,
    transform_descriptor_layout: DescriptorSetLayout,

    meshes: HashMap<MeshId, MeshState>,
    images: HashMap<ImageId, Texture>,
    materials: HashMap<MaterialId, DescriptorSet>,

    scenes: HashMap<SceneId, SceneData>,
}

impl State {
    fn new(queue: &mut CommandQueue<'_>) -> Self {
        let mesh_descriptor_layout = queue.create_descriptor_set_layout(&DescriptorSetDescriptor {
            bindings: &[
                // POSITIONS
                DescriptorBinding {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                // NORMALS
                DescriptorBinding {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                // TANGENTS
                DescriptorBinding {
                    binding: 2,
                    visibility: ShaderStages::VERTEX,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
                // UVS
                DescriptorBinding {
                    binding: 3,
                    visibility: ShaderStages::VERTEX,
                    kind: DescriptorType::Storage,
                    count: NonZeroU32::MIN,
                },
            ],
        });

        let material_descriptor_layout =
            queue.create_descriptor_set_layout(&DescriptorSetDescriptor {
                bindings: &[
                    // CONSTANTS
                    DescriptorBinding {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Uniform,
                        count: NonZeroU32::MIN,
                    },
                    // BASE_COLOR
                    DescriptorBinding {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                        count: NonZeroU32::MIN,
                    },
                    // NORMAL
                    DescriptorBinding {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                        count: NonZeroU32::MIN,
                    },
                    // METALLIC/ROUGHNESS
                    DescriptorBinding {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                        count: NonZeroU32::MIN,
                    },
                    // SPECULAR/GLOSSINESS
                    DescriptorBinding {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                        count: NonZeroU32::MIN,
                    },
                ],
            });

        let transform_descriptor_layout =
            queue.create_descriptor_set_layout(&DescriptorSetDescriptor {
                bindings: &[DescriptorBinding {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    kind: DescriptorType::Uniform,
                    count: NonZeroU32::MIN,
                }],
            });

        Self {
            default_textures: DefaultTextures::new(queue),
            mesh_descriptor_layout,
            material_descriptor_layout,
            transform_descriptor_layout,
            meshes: HashMap::new(),
            materials: HashMap::new(),
            scenes: HashMap::new(),
            images: HashMap::new(),
        }
    }
}

#[derive(Debug)]
struct DefaultTextures {
    base_color: Texture,
    normal: Texture,
    metallic_roughness: Texture,
    specular_glossiness: Texture,
}

impl DefaultTextures {
    fn new(queue: &mut CommandQueue<'_>) -> Self {
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

            texture
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
struct MeshState {
    descriptor: DescriptorSet,
    indices: IndexBuffer,
}

#[derive(Debug)]
struct SceneData {
    cameras: HashMap<CameraId, Camera>,
    objects: HashMap<ObjectId, ObjectState>,
    directional_lights: HashMap<LightId, DirectionalLight>,
    point_lights: HashMap<LightId, PointLight>,
    spot_lights: HashMap<LightId, SpotLight>,

    directional_light_buffer: Buffer,
    point_light_buffer: Buffer,
    spot_light_buffer: Buffer,
}

impl SceneData {
    fn new(queue: &mut CommandQueue<'_>) -> Self {
        Self {
            cameras: HashMap::new(),
            objects: HashMap::new(),
            directional_lights: HashMap::new(),
            point_lights: HashMap::new(),
            spot_lights: HashMap::new(),
            directional_light_buffer: queue.create_buffer_init(&BufferInitDescriptor {
                contents: DynamicBuffer::<DirectionalLightUniform>::new().as_bytes(),
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            }),
            point_light_buffer: queue.create_buffer_init(&BufferInitDescriptor {
                contents: DynamicBuffer::<PointLightUniform>::new().as_bytes(),
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            }),
            spot_light_buffer: queue.create_buffer_init(&BufferInitDescriptor {
                contents: DynamicBuffer::<SpotLightUniform>::new().as_bytes(),
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            }),
        }
    }
}

#[derive(Debug)]
struct ObjectState {
    mesh: MeshId,
    material: MaterialId,
    transform: DescriptorSet,
}
