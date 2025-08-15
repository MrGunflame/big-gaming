use bytemuck::{Pod, Zeroable};
use game_common::collections::arena::Arena;
use game_common::components::Transform;
use game_render::api::CommandQueue;
use game_render::backend::BufferUsage;
use game_render::buffer::GpuBuffer;
use game_render::buffer::slab::{CompactSlabBuffer, SlabBuffer, SlabIndex};
use game_render::buffer::suballocated::SubAllocatedGrowableBuffer;
use game_render::mesh::Mesh;
use game_tracing::trace_span;
use glam::{Mat3, Mat4, Vec4};

use crate::passes::{InstanceKey, MaterialState, MeshKey};

#[derive(Debug)]
pub struct VertexMeshState {
    pub positions: SubAllocatedGrowableBuffer<f32>,
    pub normals: SubAllocatedGrowableBuffer<f32>,
    pub uvs: SubAllocatedGrowableBuffer<f32>,
    pub tangents: SubAllocatedGrowableBuffer<f32>,
    pub colors: SubAllocatedGrowableBuffer<f32>,
    pub index_buffer: SubAllocatedGrowableBuffer<u32>,
    /// Instances that should be rendered with the opaque pipeline.
    pub opaque_instances: CompactSlabBuffer<Instance>,
    /// Instances that should be rendered with the transparent pipeline.
    pub transparent_instances: CompactSlabBuffer<Instance>,

    pub mesh_offsets: SlabBuffer<MeshOffsets>,
    meshes: Arena<MeshData>,
    pub num_opauqe_instances: u32,
    pub num_transparent_instances: u32,
}

impl VertexMeshState {
    pub fn new(queue: &CommandQueue<'_>) -> Self {
        Self {
            positions: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            normals: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            uvs: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            tangents: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            colors: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            index_buffer: SubAllocatedGrowableBuffer::new(queue, BufferUsage::INDEX),
            opaque_instances: CompactSlabBuffer::new(BufferUsage::STORAGE),
            transparent_instances: CompactSlabBuffer::new(BufferUsage::STORAGE),
            mesh_offsets: SlabBuffer::new(BufferUsage::STORAGE),
            meshes: Arena::new(),
            num_opauqe_instances: 0,
            num_transparent_instances: 0,
        }
    }

    pub fn create_mesh(&mut self, queue: &CommandQueue<'_>, mesh: &Mesh) -> MeshKey {
        let _span = trace_span!("VertexMeshState::create_mesh").entered();

        let indices = mesh.indicies().unwrap().into_u32();

        let positions_offset = self
            .positions
            .insert(queue, bytemuck::must_cast_slice(mesh.positions()));
        let normals_offset = self
            .normals
            .insert(queue, bytemuck::must_cast_slice(mesh.normals()));
        let uvs_offset = self
            .uvs
            .insert(queue, bytemuck::must_cast_slice(mesh.uvs()));
        let tangents_offset = self
            .tangents
            .insert(queue, bytemuck::must_cast_slice(mesh.tangents()));

        // If no colors are set we set all colors to 1.0
        // which corresponds to no effect.
        // TODO: Instead of duplicating a bunch of 1.0s for each mesh
        // we should share them between meshes.
        let mut colors = mesh.colors().to_vec();
        colors.resize(mesh.positions().len(), Vec4::ONE);

        let colors_offset = self
            .colors
            .insert(queue, bytemuck::must_cast_slice::<Vec4, f32>(&colors));

        let indices_offset = self.index_buffer.insert(queue, &indices);

        let offsets = self.mesh_offsets.insert(&MeshOffsets {
            positions_offset: positions_offset as u32,
            normals_offset: normals_offset as u32,
            uvs_offset: uvs_offset as u32,
            tangents_offset: tangents_offset as u32,
            colors_offset: colors_offset as u32,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        });

        MeshKey(self.meshes.insert(MeshData {
            positions_offset,
            normals_offset,
            uvs_offset,
            tangents_offset,
            indices_offset,
            colors_offset,
            indices_count: indices.len() as u64,
            offsets,
        }))
    }

    pub fn remove_mesh(&mut self, key: MeshKey) {
        if let Some(mesh) = self.meshes.remove(key.0) {
            self.positions.remove(mesh.positions_offset);
            self.normals.remove(mesh.normals_offset);
            self.uvs.remove(mesh.uvs_offset);
            self.tangents.remove(mesh.tangents_offset);
            self.colors.remove(mesh.colors_offset);
            self.index_buffer.remove(mesh.indices_offset);
            self.mesh_offsets.remove(mesh.offsets);
        }
    }

    pub fn create_instance(
        &mut self,
        transform: Transform,
        mesh: MeshKey,
        material: &MaterialState,
    ) -> InstanceKey {
        let mesh = self.meshes.get(mesh.0).unwrap();

        let normal = Mat3::from_quat(transform.rotation);
        let normal_x = Vec4::new(normal.x_axis.x, normal.x_axis.y, normal.x_axis.z, 0.0);
        let normal_y = Vec4::new(normal.y_axis.x, normal.y_axis.y, normal.y_axis.z, 0.0);
        let normal_z = Vec4::new(normal.z_axis.x, normal.z_axis.y, normal.z_axis.z, 0.0);

        if material.is_opaque {
            self.num_opauqe_instances += 1;
            InstanceKey::Opaque(
                self.opaque_instances.insert(&Instance {
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
                    material_index: material.index,
                    offsets: mesh.offsets,
                    index_offset: mesh.indices_offset as u32,
                    index_count: mesh.indices_count as u32,
                }),
            )
        } else {
            self.num_transparent_instances += 1;
            InstanceKey::Transparent(
                self.transparent_instances.insert(&Instance {
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
                    material_index: material.index,
                    offsets: mesh.offsets,
                    index_offset: mesh.indices_offset as u32,
                    index_count: mesh.indices_count as u32,
                }),
            )
        }
    }

    pub fn remove_instance(&mut self, key: InstanceKey) {
        match key {
            InstanceKey::Opaque(key) => {
                self.opaque_instances.remove(key);
                self.num_opauqe_instances -= 1;
            }
            InstanceKey::Transparent(key) => {
                self.transparent_instances.remove(key);
                self.num_transparent_instances -= 1;
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Instance {
    transform: [[f32; 4]; 4],
    // rotation matrix for normals/tangents
    // Note that we can't use the transform matrix for non-uniform
    // scaling values.
    normal: [[f32; 4]; 3],
    material_index: SlabIndex,
    offsets: SlabIndex,
    index_offset: u32,
    index_count: u32,
}

impl GpuBuffer for Instance {
    const ALIGN: usize = 16;
    const SIZE: usize = size_of::<Self>();
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct MeshOffsets {
    positions_offset: u32,
    normals_offset: u32,
    uvs_offset: u32,
    tangents_offset: u32,
    colors_offset: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

impl GpuBuffer for MeshOffsets {
    const ALIGN: usize = 4;
    const SIZE: usize = size_of::<Self>();
}

#[derive(Copy, Clone, Debug)]
struct MeshData {
    positions_offset: u64,
    normals_offset: u64,
    uvs_offset: u64,
    tangents_offset: u64,
    colors_offset: u64,
    indices_offset: u64,
    indices_count: u64,
    offsets: SlabIndex,
}
