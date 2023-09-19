use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{Buffer, BufferUsages, Device, Queue};

use crate::buffer::DynamicBuffer;
use crate::camera::CameraBuffer;
use crate::entities::SceneEntities;
use crate::forward::ForwardPipeline;
use crate::light::pipeline::{DirectionalLightUniform, PointLightUniform, SpotLightUniform};
use crate::mipmap::MipMapGenerator;
use crate::pbr::material::Materials;
use crate::pbr::mesh::Meshes;
use crate::render_pass::GpuObject;
use crate::texture::Images;

pub(crate) struct RenderState {
    pub entities: SceneEntities,
    pub cameras: Vec<CameraBuffer>,
    pub directional_lights: Buffer,
    pub point_lights: Buffer,
    pub spot_lights: Buffer,
    pub objects: Vec<GpuObject>,
}

impl RenderState {
    pub fn new(
        device: &Device,
        queue: &Queue,
        meshes: &Meshes,
        materials: &Materials,
        images: &Images,
        entities: &SceneEntities,
        pipeline: &ForwardPipeline,
        mipmap_generator: &mut MipMapGenerator,
    ) -> Self {
        let buffer = DynamicBuffer::<DirectionalLightUniform>::new();
        let directional_lights = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: buffer.as_bytes(),
            usage: BufferUsages::STORAGE,
        });

        let buffer = DynamicBuffer::<PointLightUniform>::new();
        let point_lights = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: buffer.as_bytes(),
            usage: BufferUsages::STORAGE,
        });

        let buffer = DynamicBuffer::<SpotLightUniform>::new();
        let spot_lights = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: buffer.as_bytes(),
            usage: BufferUsages::STORAGE,
        });

        let cameras = entities
            .cameras
            .values()
            .map(|camera| {
                CameraBuffer::new(
                    camera.transform,
                    camera.projection,
                    device,
                    camera.target.clone(),
                )
            })
            .collect();

        let mut objects = Vec::new();
        for obj in &mut entities.objects.values() {
            let Some(mesh) = meshes.get(obj.mesh) else {
                tracing::warn!("no such mesh id: {:?}", obj.mesh);
                continue;
            };

            let Some(material) = materials.get(obj.material) else {
                tracing::warn!("no such material id: {:?}", obj.material);
                continue;
            };

            let (mesh_bg, indices) =
                crate::pbr::mesh::update_mesh_bind_group(device, pipeline, mesh);

            let mat_bg = crate::pbr::material::update_material_bind_group(
                device,
                queue,
                images,
                pipeline,
                material,
                mipmap_generator,
            );

            let transform = crate::pbr::mesh::update_transform_buffer(obj.transform, device);

            objects.push(GpuObject {
                material_bind_group: mat_bg,
                mesh_bind_group: mesh_bg,
                transform,
                indices,
            });
        }

        Self {
            entities: entities.clone(),
            directional_lights,
            spot_lights,
            point_lights,
            cameras,
            objects,
        }
    }

    pub fn empty(device: &Device) -> Self {
        let buffer = DynamicBuffer::<DirectionalLightUniform>::new();
        let directional_lights = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: buffer.as_bytes(),
            usage: BufferUsages::STORAGE,
        });

        let buffer = DynamicBuffer::<PointLightUniform>::new();
        let point_lights = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: buffer.as_bytes(),
            usage: BufferUsages::STORAGE,
        });

        let buffer = DynamicBuffer::<SpotLightUniform>::new();
        let spot_lights = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: buffer.as_bytes(),
            usage: BufferUsages::STORAGE,
        });

        Self {
            directional_lights,
            point_lights,
            spot_lights,
            objects: vec![],
            cameras: vec![],
            entities: SceneEntities::new(),
        }
    }
}
