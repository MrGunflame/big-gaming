use game_tracing::trace_span;
use glam::{Vec2, Vec3, Vec4};
use mikktspace::Geometry;

use crate::aabb::Aabb;

// FIXME: Meshes will be duplicated quite a bit, so
// we don't want to have it attached to every entity.
#[derive(Clone, Debug)]
pub struct Mesh {
    indices: Option<Indices>,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    uvs: Vec<Vec2>,
    tangents: Vec<Vec4>,
    tangents_set: bool,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            indices: None,
            positions: vec![],
            normals: vec![],
            uvs: vec![],
            tangents: vec![],
            tangents_set: false,
        }
    }

    pub fn set_indices(&mut self, indices: Indices) {
        self.indices = Some(indices);
    }

    pub fn set_positions(&mut self, positions: Vec<Vec3>) {
        self.positions = positions;
    }

    pub fn positions(&self) -> &[Vec3] {
        &self.positions
    }

    pub fn set_normals(&mut self, normals: Vec<Vec3>) {
        self.normals = normals;
    }

    pub fn normals(&self) -> &[Vec3] {
        &self.normals
    }

    pub fn tangents(&self) -> &[Vec4] {
        &self.tangents
    }

    pub fn set_tangents(&mut self, tangents: Vec<Vec4>) {
        self.tangents = tangents;
        self.tangents_set = true;
    }

    pub fn set_uvs(&mut self, uvs: Vec<Vec2>) {
        self.uvs = uvs;
    }

    pub fn uvs(&self) -> &[Vec2] {
        &self.uvs
    }

    pub fn indicies(&self) -> Option<Indices> {
        self.indices.clone()
    }

    pub fn compute_tangents(&mut self) {
        let _span = trace_span!("Mesh::compute_tangents").entered();

        // TODO: Checks and precomputes should move to gltf crate.
        assert_eq!(self.positions.len(), self.normals.len());
        assert_eq!(self.positions.len(), self.uvs.len());

        let len = self.positions.len();

        let mut mesh = Mikktpsace {
            indices: self.indices.as_ref().unwrap().as_u32(),
            positions: &self.positions,
            normals: &self.normals,
            uvs: &self.uvs,
            tangents: vec![Vec4::ZERO; len],
        };
        mikktspace::generate_tangents(&mut mesh);

        self.tangents = mesh.tangents;
    }

    pub fn tangents_set(&self) -> bool {
        self.tangents_set
    }

    pub fn compute_aabb(&self) -> Option<Aabb> {
        // We need at least one vertex to determine an AABB.
        if self.positions.is_empty() {
            return None;
        }

        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for pos in &self.positions {
            min = Vec3::min(min, *pos);
            max = Vec3::max(max, *pos);
        }

        Some(Aabb::from_min_max(min, max))
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub enum Indices {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl Indices {
    pub fn len(&self) -> u32 {
        match self {
            Self::U16(buf) => buf.len() as u32,
            Self::U32(buf) => buf.len() as u32,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_u32(&self) -> &[u32] {
        match self {
            Self::U32(val) => val,
            _ => panic!("`Indicies` is not `U32`"),
        }
    }

    pub fn as_u16(&self) -> &[u16] {
        match self {
            Self::U16(val) => val,
            _ => panic!("`Indices` is not `U16`"),
        }
    }

    pub fn into_u32(self) -> Vec<u32> {
        match self {
            Self::U16(val) => val.into_iter().map(u32::from).collect(),
            Self::U32(val) => val,
        }
    }
}

struct Mikktpsace<'a> {
    indices: &'a [u32],
    positions: &'a [Vec3],
    normals: &'a [Vec3],
    uvs: &'a [Vec2],
    tangents: Vec<Vec4>,
}

impl<'a> Mikktpsace<'a> {
    fn index(&self, face: usize, vert: usize) -> usize {
        self.indices[face * 3 + vert] as usize
    }
}

impl<'a> Geometry for Mikktpsace<'a> {
    fn num_faces(&self) -> usize {
        self.indices.len() / 3
    }

    fn num_vertices_of_face(&self, _face: usize) -> usize {
        3
    }

    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        self.positions[self.index(face, vert)].to_array()
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        self.normals[self.index(face, vert)].to_array()
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        self.uvs[self.index(face, vert)].to_array()
    }

    fn set_tangent_encoded(&mut self, tangent: [f32; 4], face: usize, vert: usize) {
        let tangent = Vec4::from_array(tangent);
        let index = self.index(face, vert);
        self.tangents[index] = tangent;
    }
}
