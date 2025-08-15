use bytemuck::{Pod, Zeroable};
use game_common::collections::arena::Arena;
use game_common::components::Transform;
use game_render::api::CommandQueue;
use game_render::backend::BufferUsage;
use game_render::buffer::GpuBuffer;
use game_render::buffer::slab::{CompactSlabBuffer, SlabIndex};
use game_render::buffer::suballocated::SubAllocatedGrowableBuffer;
use game_render::mesh::Mesh;
use game_tracing::trace_span;
use glam::{Mat3, Mat4, Vec4};
use meshopt::VertexDataAdapter;

use crate::passes::{InstanceKey, MeshKey};

const VERTEX_COUNT: usize = 128;
const TRIANGLE_COUNT: usize = 256;

#[derive(Debug)]
pub struct MeshState {
    pub positions: SubAllocatedGrowableBuffer<f32>,
    pub normals: SubAllocatedGrowableBuffer<f32>,
    pub uvs: SubAllocatedGrowableBuffer<f32>,
    pub tangents: SubAllocatedGrowableBuffer<f32>,
    pub vertex_indices: SubAllocatedGrowableBuffer<u32>,
    pub triangle_indices: SubAllocatedGrowableBuffer<u8>,
    pub meshlets: SubAllocatedGrowableBuffer<Meshlet>,
    pub instances: CompactSlabBuffer<Instance>,
    meshes: Arena<MeshData>,
}

impl MeshState {
    pub fn new(queue: &CommandQueue<'_>) -> Self {
        Self {
            positions: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            normals: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            uvs: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            tangents: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            vertex_indices: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            triangle_indices: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            meshlets: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            instances: CompactSlabBuffer::new(BufferUsage::STORAGE),
            meshes: Arena::new(),
        }
    }

    pub fn create_mesh(&mut self, queue: &CommandQueue<'_>, mesh: &Mesh) -> MeshKey {
        let _span = trace_span!("MeshState::create_mesh").entered();

        let indices = mesh.indicies().unwrap().into_u32();
        let vertices = VertexDataAdapter::new(
            bytemuck::must_cast_slice(mesh.positions()),
            size_of::<f32>() * 3,
            0,
        )
        .unwrap();

        let meshlets = meshopt::clusterize::build_meshlets(
            &indices,
            &vertices,
            VERTEX_COUNT,
            TRIANGLE_COUNT,
            0.0,
        );

        let positions_offset = self
            .positions
            .insert(queue, bytemuck::cast_slice(mesh.positions()));
        let normals_offset = self
            .normals
            .insert(queue, bytemuck::cast_slice(mesh.normals()));
        let uvs_offset = self.uvs.insert(queue, bytemuck::cast_slice(mesh.uvs()));
        let tangents_offset = self
            .tangents
            .insert(queue, bytemuck::cast_slice(mesh.tangents()));

        let vertex_indices_offset = self.vertex_indices.insert(queue, &meshlets.vertices);
        let triangle_indices_offset = self.triangle_indices.insert(queue, &meshlets.triangles);

        let meshlets = meshlets
            .meshlets
            .iter()
            .map(|meshlet| {
                debug_assert!(
                    meshlet.vertex_offset as usize + meshlet.vertex_count as usize
                        <= meshlets.vertices.len()
                );
                debug_assert!(
                    meshlet.triangle_offset as usize + meshlet.triangle_count as usize * 3
                        <= meshlets.triangles.len()
                );

                Meshlet {
                    positions_offset: positions_offset as u32,
                    normals_offset: normals_offset as u32,
                    uvs_offset: uvs_offset as u32,
                    tangents_offset: tangents_offset as u32,
                    vertex_offset: vertex_indices_offset as u32 + meshlet.vertex_offset,
                    vertex_count: meshlet.vertex_count,
                    triangle_offset: triangle_indices_offset as u32 + meshlet.triangle_offset,
                    triangle_count: meshlet.triangle_count,
                }
            })
            .collect::<Vec<_>>();

        let meshlets_offset = self.meshlets.insert(queue, &meshlets);

        MeshKey(self.meshes.insert(MeshData {
            positions: positions_offset,
            normals: normals_offset,
            uvs: uvs_offset,
            tangents: tangents_offset,
            meshlets_offset,
            meshlet_count: meshlets.len() as u32,
        }))
    }

    pub fn create_instance(
        &mut self,
        transform: Transform,
        mesh: MeshKey,
        material_index: SlabIndex,
    ) -> InstanceKey {
        let meshlet_offset = self.meshes.get(mesh.0).unwrap().meshlets_offset as u32;
        let meshlet_count = self.meshes.get(mesh.0).unwrap().meshlet_count as u32;

        let normal = Mat3::from_quat(transform.rotation);
        let normal_x = Vec4::new(normal.x_axis.x, normal.x_axis.y, normal.x_axis.z, 0.0);
        let normal_y = Vec4::new(normal.y_axis.x, normal.y_axis.y, normal.y_axis.z, 0.0);
        let normal_z = Vec4::new(normal.z_axis.x, normal.z_axis.y, normal.z_axis.z, 0.0);

        InstanceKey::Opaque(
            self.instances.insert(&Instance {
                transform: Mat4::from_scale_rotation_translation(
                    transform.scale,
                    transform.rotation,
                    transform.translation,
                )
                .to_cols_array_2d(),
                normal: [
                    normal_x.to_array(),
                    normal_y.to_array(),
                    normal_z.to_array(),
                ],
                meshlet_offset,
                meshlet_count,
                material_index: material_index,
                _pad0: 0,
            }),
        )
    }

    pub fn remove_instance(&mut self, id: InstanceKey) {
        match id {
            InstanceKey::Opaque(id) => {
                self.instances.remove(id);
            }
            _ => unreachable!(),
        }
    }

    pub fn get_meshlet_offset(&self, id: MeshKey) -> Option<u32> {
        self.meshes.get(id.0).map(|m| m.meshlets_offset as u32)
    }

    pub fn get_meshlet_count(&self, id: MeshKey) -> Option<u32> {
        self.meshes.get(id.0).map(|m| m.meshlet_count)
    }

    pub fn remove_mesh(&mut self, id: MeshKey) {
        if let Some(mesh) = self.meshes.remove(id.0) {
            self.positions.remove(mesh.positions);
            self.normals.remove(mesh.normals);
            self.uvs.remove(mesh.uvs);
            self.tangents.remove(mesh.tangents);
            self.meshlets.remove(mesh.meshlets_offset);
        }
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Meshlet {
    pub vertex_offset: u32,
    pub triangle_offset: u32,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub positions_offset: u32,
    pub normals_offset: u32,
    pub uvs_offset: u32,
    pub tangents_offset: u32,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Instance {
    pub transform: [[f32; 4]; 4],
    // rotation matrix for normals/tangents
    // Note that we can't use the transform matrix for non-uniform
    // scaling values.
    pub normal: [[f32; 4]; 3],
    pub meshlet_offset: u32,
    pub meshlet_count: u32,
    pub material_index: SlabIndex,
    _pad0: u32,
}

impl GpuBuffer for Instance {
    const SIZE: usize = size_of::<Self>();
    const ALIGN: usize = 16;
}

#[derive(Copy, Clone, Debug)]
struct MeshData {
    positions: u64,
    normals: u64,
    uvs: u64,
    tangents: u64,
    meshlets_offset: u64,
    meshlet_count: u32,
}

#[derive(Copy, Clone, Debug)]
struct InstanceData {
    index: u64,
}
