#[cfg(feature = "render")]
use bevy_render::mesh::Indices;
#[cfg(feature = "render")]
use bevy_render::prelude::Mesh;
#[cfg(feature = "render")]
use bevy_render::render_resource::PrimitiveTopology;

use glam::{UVec2, Vec3};
use image::{GenericImageView, Luma, Primitive};

use super::{CellId, CELL_SIZE, CELL_SIZE_UINT};

#[derive(Clone, Debug, PartialEq)]
pub struct TerrainMesh {
    pub cell: CellId,
    offsets: Heightmap,
}

impl TerrainMesh {
    pub fn new(cell: CellId, offsets: Heightmap) -> Self {
        Self { cell, offsets }
    }

    pub fn height(&self) -> &Heightmap {
        &self.offsets
    }

    pub fn verts_indices(&self) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let size_x = CELL_SIZE_UINT.x + 1;
        let size_z = CELL_SIZE_UINT.z + 1;

        let projection = Projection::new(&self.offsets, UVec2::new(size_x, size_z));

        for index in 0u32..size_x * size_z {
            let x = index / size_x;
            let z = index % size_z;

            let y = projection.get(x, z);

            vertices.push(Vec3::new(x as f32, y as f32, z as f32));

            if x != size_x - 1 && z != size_z - 1 {
                // Up tri (index -> index + 10 -> index + 10 + 1)
                indices.push([index, index + size_x, index + size_x + 1]);

                // Down tri (index -> index + 1 -> index + 10 + 1)
                indices.push([index + size_x + 1, index + 1, index]);
            }
        }

        (vertices, indices)
    }

    #[cfg(feature = "render")]
    pub fn mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();

        let size_x = CELL_SIZE_UINT.x + 1;
        let size_z = CELL_SIZE_UINT.z + 1;

        let projection = Projection::new(&self.offsets, UVec2::new(size_x, size_z));

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

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.set_indices(Some(Indices::U32(indices)));
        // mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

        // mesh.duplicate_vertices();
        // mesh.compute_flat_normals();

        // dbg!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL));
        // panic!();

        mesh
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Heightmap {
    size: UVec2,
    nodes: Vec<f32>,
}

impl Heightmap {
    pub fn from_vec(size: UVec2, nodes: Vec<f32>) -> Self {
        assert!(nodes.len() as u32 / size.x == size.y);

        Self { size, nodes }
    }

    pub fn size(&self) -> UVec2 {
        self.size
    }

    pub fn nodes(&self) -> &[f32] {
        &self.nodes
    }

    pub fn from_image<T, P>(view: T) -> Self
    where
        T: GenericImageView<Pixel = Luma<P>>,
        P: Primitive,
    {
        let (x, y) = view.dimensions();

        let nodes = view
            .pixels()
            .map(|(_, _, p)| {
                let p = p.0[0];
                let v = p.to_f32().unwrap();

                // P::MAX is the bounds of the cell.
                let res = v / (P::DEFAULT_MAX_VALUE.to_f32().unwrap() / CELL_SIZE.y);
                debug_assert!(res <= CELL_SIZE.y);
                res
            })
            .collect();

        Self::from_vec(UVec2::new(x, y), nodes)
    }

    pub fn get(&self, x: u32, y: u32) -> f32 {
        assert!(x < self.size.x && y < self.size.y);

        // Wrong axes
        let index = y * self.size.x + x;

        self.nodes[index as usize]
    }
}

impl AsRef<[f32]> for Heightmap {
    #[inline]
    fn as_ref(&self) -> &[f32] {
        &self.nodes
    }
}

impl<T, P> From<T> for Heightmap
where
    T: GenericImageView<Pixel = Luma<P>>,
    P: Primitive,
{
    fn from(value: T) -> Self {
        Self::from_image(value)
    }
}

/// A projection of a Heightmap onto a different sized plane.
struct Projection<'a> {
    heightmap: &'a Heightmap,
    dimensions: UVec2,
}

impl<'a> Projection<'a> {
    #[inline]
    fn new(heightmap: &'a Heightmap, dimensions: UVec2) -> Self {
        Self {
            heightmap,
            dimensions,
        }
    }

    fn get(&self, x: u32, y: u32) -> f32 {
        assert!(x < self.dimensions.x && y < self.dimensions.y);

        let xf = x as f32 / (self.dimensions.x as f32 - 1.0);
        let yf = y as f32 / (self.dimensions.y as f32 - 1.0);

        // Bilinear interpolation
        let w = self.heightmap.size.x as f32 - 1.0;
        let h = self.heightmap.size.y as f32 - 1.0;

        let x1 = f32::floor(xf * w);
        let y1 = f32::floor(yf * h);
        let x2 = f32::clamp(x1 + 1.0, 0.0, w);
        let y2 = f32::clamp(y1 + 1.0, 0.0, h);

        let xp = xf * w - x1;
        let yp = yf * h - y1;

        let p11 = self.heightmap.get(x1 as u32, y1 as u32);
        let p21 = self.heightmap.get(x2 as u32, y1 as u32);
        let p12 = self.heightmap.get(x1 as u32, y2 as u32);
        let p22 = self.heightmap.get(x2 as u32, y2 as u32);

        let px1 = lerp(p11, p21, xp);
        let px2 = lerp(p12, p22, xp);

        lerp(px1, px2, yp)
    }
}

fn lerp(lhs: f32, rhs: f32, s: f32) -> f32 {
    lhs + ((rhs - lhs) * s)
}

#[cfg(test)]
mod tests {
    use glam::UVec2;

    use super::{Heightmap, Projection};

    #[test]
    fn test_heightmap() {
        let nodes = vec![0.0, 1.0, 2.0, 3.0];
        let map = Heightmap::from_vec(UVec2::new(2, 2), nodes);

        assert_eq!(map.get(0, 0), 0.0);
        assert_eq!(map.get(1, 0), 1.0);
        assert_eq!(map.get(0, 1), 2.0);
        assert_eq!(map.get(1, 1), 3.0);
    }

    #[test]
    fn test_projection() {
        let nodes = vec![
            0.0, 1.0, 2.0, // 0
            0.0, 1.0, 2.0, // 1
            0.0, 1.0, 2.0, // 2
        ];
        let map = Heightmap::from_vec(UVec2::new(3, 3), nodes);

        let proj = Projection::new(&map, UVec2::new(5, 5));

        assert_eq!(proj.get(0, 0), 0.0);
        assert_eq!(proj.get(1, 0), 0.5);
        assert_eq!(proj.get(2, 0), 1.0);
        assert_eq!(proj.get(3, 0), 1.5);
        assert_eq!(proj.get(4, 0), 2.0);
    }
}
