use game_asset::Asset;
use glam::{Vec2, Vec3, Vec4};

use crate::aabb::Aabb;

// FIXME: Meshes will be duplicated quite a bit, so
// we don't want to have it attached to every entity.
#[derive(Clone, Debug)]
pub struct Mesh {
    indices: Option<Indices>,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
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

    pub fn set_positions(&mut self, positions: Vec<[f32; 3]>) {
        self.positions = positions;
    }

    pub fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }

    pub fn set_normals(&mut self, normals: Vec<[f32; 3]>) {
        self.normals = normals;
    }

    pub fn normals(&self) -> &[[f32; 3]] {
        &self.normals
    }

    pub fn tangents(&self) -> &[Vec4] {
        &self.tangents
    }

    pub fn set_tangents(&mut self, tangents: Vec<Vec4>) {
        self.tangents = tangents;
        self.tangents_set = true;
    }

    pub fn set_uvs(&mut self, uvs: Vec<[f32; 2]>) {
        self.uvs = uvs;
    }

    pub fn uvs(&self) -> &[[f32; 2]] {
        &self.uvs
    }

    pub fn indicies(&self) -> Option<Indices> {
        self.indices.clone()
    }

    pub fn compute_tangents(&mut self) {
        let mut triangles_included = vec![];

        self.tangents.clear();

        let len = self.positions.len();

        self.tangents.resize(len, Vec4::new(0.0, 0.0, 0.0, 1.0));
        triangles_included.resize(len, 0);

        for c in self.indices.clone().unwrap().into_u32().chunks(3) {
            let pos0 = Vec3::from_array(self.positions[c[0] as usize]);
            let pos1 = Vec3::from_array(self.positions[c[1] as usize]);
            let pos2 = Vec3::from_array(self.positions[c[2] as usize]);

            let uv0 = Vec2::from_array(self.uvs[c[0] as usize]);
            let uv1 = Vec2::from_array(self.uvs[c[1] as usize]);
            let uv2 = Vec2::from_array(self.uvs[c[2] as usize]);

            let delta_pos1 = pos1 - pos0;
            let delta_pos2 = pos2 - pos0;

            let delta_uv1 = uv1 - uv0;
            let delta_uv2 = uv2 - uv0;

            let f = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
            let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * f;

            // Note that the orientation is already set to 1.0 on every tangent, we don't
            // want to change that.
            let tangent_summand = Vec4::new(tangent.x, tangent.y, tangent.z, 0.0);

            self.tangents[c[0] as usize] += tangent_summand;
            self.tangents[c[1] as usize] += tangent_summand;
            self.tangents[c[2] as usize] += tangent_summand;

            triangles_included[c[0] as usize] += 1;
            triangles_included[c[1] as usize] += 1;
            triangles_included[c[2] as usize] += 1;
        }

        // Average Tangents/Bitangents
        for (i, &n) in triangles_included.iter().enumerate() {
            debug_assert_ne!(n, 0);

            let denom = 1.0 / n as f32;

            // Don't change the W component.
            let x = self.tangents[i].x * denom;
            let y = self.tangents[i].y * denom;
            let z = self.tangents[i].z * denom;

            self.tangents[i] = Vec4::new(x, y, z, self.tangents[i].w);
        }

        self.tangents_set = true;
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
            min = Vec3::min(min, Vec3::from_slice(pos));
            max = Vec3::max(max, Vec3::from_slice(pos));
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

impl Asset for Mesh {}

#[cfg(test)]
mod tests {
    use glam::Vec4;

    use super::{Indices, Mesh};

    #[test]
    fn mesh_computed_tangents() {
        let mut mesh = Mesh::new();
        mesh.set_positions(vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]);
        mesh.set_uvs(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
        mesh.set_indices(Indices::U32(vec![0, 1, 2]));
        mesh.set_normals(vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]]);

        mesh.compute_tangents();

        assert_eq!(
            mesh.tangents,
            vec![
                Vec4::from_array([1.0, 0.0, 0.0, 1.0]),
                Vec4::from_array([1.0, 0.0, 0.0, 1.0]),
                Vec4::from_array([1.0, 0.0, 0.0, 1.0]),
            ]
        );
    }
}
