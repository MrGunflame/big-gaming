use game_common::components::{Color, Transform};
use game_core::hierarchy::Hierarchy;
use glam::{Vec2, Vec3, Vec4};
use gltf::material::AlphaMode;

#[derive(Clone, Debug)]
pub struct GltfScene {
    pub nodes: Hierarchy<GltfNode>,
}

#[derive(Clone, Debug)]
pub struct GltfNode {
    pub transform: Transform,
    pub mesh: Option<MeshIndex>,
    pub material: Option<MaterialIndex>,
    pub name: Option<String>,
}

#[derive(Copy, Clone, Debug)]
pub struct GltfMeshMaterial {
    pub mesh: MeshIndex,
    pub material: MaterialIndex,
}

#[derive(Clone, Debug, Default)]
pub struct GltfMesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub tangents: Vec<Vec4>,
    pub indices: Vec<u32>,
}

#[derive(Copy, Clone, Debug)]
pub struct GltfMaterial {
    pub alpha_mode: AlphaMode,
    pub base_color: Color,
    pub base_color_texture: Option<TextureIndex>,
    pub normal_texture: Option<TextureIndex>,
    pub roughness: f32,
    pub metallic: f32,
    pub metallic_roughness_texture: Option<TextureIndex>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshIndex {
    // A `MeshIndex` is composed of the gltf mesh and primitive index.
    // The primitive is what our scene "mesh" actually is.
    // Neither index is enough to uniquely identify the mesh.
    pub(crate) mesh: usize,
    pub(crate) primitive: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialIndex(pub(crate) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureIndex(pub(crate) usize);
