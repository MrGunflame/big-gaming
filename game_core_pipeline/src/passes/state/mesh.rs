use bytemuck::{Pod, Zeroable};
use game_common::collections::arena::{Arena, Key};
use game_render::api::CommandQueue;
use game_render::backend::BufferUsage;
use game_render::buffer::suballocated::SubAllocatedGrowableBuffer;
use game_render::mesh::Mesh;
use meshopt::VertexDataAdapter;

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
    meshes: Arena<MeshData>,
}

impl MeshState {
    pub fn new(queue: &mut CommandQueue<'_>) -> Self {
        Self {
            positions: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            normals: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            uvs: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            tangents: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            vertex_indices: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            triangle_indices: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            meshlets: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            meshes: Arena::new(),
        }
    }

    pub fn create_mesh(&mut self, queue: &mut CommandQueue<'_>, mesh: &Mesh) -> MeshStrategyMeshId {
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

        MeshStrategyMeshId(self.meshes.insert(MeshData {
            positions: positions_offset,
            normals: normals_offset,
            uvs: uvs_offset,
            tangents: tangents_offset,
            meshlets_offset,
            meshlet_count: meshlets.len() as u32,
        }))
    }

    pub fn get_meshlet_offset(&self, id: MeshStrategyMeshId) -> Option<u32> {
        self.meshes.get(id.0).map(|m| m.meshlets_offset as u32)
    }

    pub fn get_meshlet_count(&self, id: MeshStrategyMeshId) -> Option<u32> {
        self.meshes.get(id.0).map(|m| m.meshlet_count)
    }

    pub fn remove(&mut self, id: MeshStrategyMeshId) {
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshStrategyMeshId(pub Key);

#[derive(Copy, Clone, Debug)]
struct MeshData {
    positions: u64,
    normals: u64,
    uvs: u64,
    tangents: u64,
    meshlets_offset: u64,
    meshlet_count: u32,
}
