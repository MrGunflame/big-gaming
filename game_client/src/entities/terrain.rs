use game_common::components::transform::Transform;
use game_common::world::entity::Terrain;
use game_common::world::terrain::{Projection, TerrainMesh};
use game_common::world::CELL_SIZE_UINT;
use game_core::hierarchy::Hierarchy;
use game_render::mesh::{Indices, Mesh};
use game_render::Renderer;
use game_scene::scene::{Material, Node, NodeBody, ObjectNode, Scene};
use game_scene::scene2::Key;
use game_tracing::trace_span;
use glam::{UVec2, Vec3};

use crate::scene::SceneState;

#[derive(Clone, Debug)]
pub struct LoadTerrain {
    pub terrain: Terrain,
}

pub fn spawn_terrain(
    scenes: &mut SceneState,
    renderer: &mut Renderer,
    terrain: &TerrainMesh,
    key: Key,
) {
    let _span = trace_span!("spawn_terrain").entered();

    let mesh = build_mesh(terrain);

    let mut nodes = Hierarchy::new();
    nodes.append(
        None,
        Node {
            transform: Transform::default(),
            body: NodeBody::Object(ObjectNode {
                mesh: 0,
                material: 0,
            }),
        },
    );

    scenes.spawner.insert(
        key,
        Scene {
            nodes,
            meshes: vec![mesh],
            materials: vec![Material::default()],
            images: vec![],
        },
    );
}

fn build_mesh(terrain: &TerrainMesh) -> Mesh {
    let mut mesh = Mesh::new();

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();

    let size_x = CELL_SIZE_UINT.x + 1;
    let size_z = CELL_SIZE_UINT.z + 1;

    let projection = Projection::new(&terrain.offsets, UVec2::new(size_x, size_z));

    for index in 0u32..size_x * size_z {
        let x = index % size_x;
        let z = index / size_z;

        let y = projection.get(x, z);

        vertices.push([x as f32, y as f32, z as f32]);
        // normals.push([0.0, 0.0, 1.0]);

        if x != size_x - 1 && z != size_z - 1 {
            // Up tri (index -> index + 10 -> index + 10 + 1)
            indices.extend([index, index + size_x, index + size_x + 1]);

            // Down tri (index -> index + 1 -> index + 10 + 1)
            indices.extend([index + size_x + 1, index + 1, index]);
        }
    }

    for index in 0u32..size_x * size_z {
        let x = index % size_x;
        let z = index / size_z;

        if x != size_x - 1 && z != size_z - 1 {
            let x: Vec3 = vertices[index as usize + size_x as usize].into();
            let z: Vec3 = vertices[index as usize + 1].into();

            let face_normal = x.cross(z).normalize();

            normals.push(face_normal);

            // Outer edges
        } else {
            normals.push(Vec3::new(0.0, 1.0, 0.0));
        }
    }

    for index in 0..size_x * size_z {
        let x = ((index % size_x) as f32) * (1f32 / (size_x as f32 - 1.0));
        let z = ((index / size_z) as f32) * (1f32 / (size_z as f32 - 1.0));
        uvs.push([x as f32, z as f32]);
    }

    // for index in 0u32..size_x * size_z {
    //     let x = index % size_x;
    //     let z = index / size_z;

    //     if x != size_x && s != size_z {
    //         let normal = vertices[index + size_x + 1] - vertices[index];
    //     }
    // }

    // let mut index = 0;
    // assert!(vertices.len() % 3 == 0);
    // while index < vertices.len() {
    //     let a = vertices[index];
    //     let b = vertices[index + 1];
    //     let c = vertices[index + 2];

    //     let (a, b, c) = (Vec3::from(a), Vec3::from(b), Vec3::from(c));
    //     let normal: [f32; 3] = (b - a).cross(c - a).normalize().into();

    //     normals.push([0.0, 0.0, 0.0]);

    //     normals.extend([normal, normal, normal]);

    //     index += 3;
    // }

    // for index in 0u32..vertices {
    //     let x = index % size_x;
    //     let z = index / size_z;

    //     if x == size_x - 1 || z == size_z - 1 {
    //         continue;
    //     }

    //     // Up tri
    //     let a = vertices[index as usize];
    //     let b = vertices[index as usize + size_x as usize];
    //     let c = vertices[index as usize + size_x as usize + 1];

    //     let (a, b, c) = (Vec3::from(a), Vec3::from(b), Vec3::from(c));
    //     let normal: [f32; 3] = (b - a).cross(c - a).normalize().into();

    //     normals.push(normal);

    //     // Down tri
    //     // let a = vertices[index as usize + size_x as usize + 1];
    //     // let b = vertices[index as usize + 1];
    //     // let c = vertices[index as usize];
    //     // dbg!((a, b, c));

    //     // let (a, b, c) = (Vec3::from(a), Vec3::from(b), Vec3::from(c));
    //     // let normal: [f32; 3] = (b - a).cross(c - a).normalize().into();

    //     // normals.push(normal);
    // }

    let normals: Vec<[f32; 3]> = (0..vertices.len()).map(|_| [0.0, 0.0, 1.0]).collect();

    mesh.set_positions(vertices);
    mesh.set_indices(Indices::U32(indices));
    mesh.set_uvs(uvs);
    mesh.set_normals(normals);
    mesh.compute_tangents();

    mesh
}
