use game_model::buffer::Buffer;
use game_model::material::Material;
use game_model::mesh::Mesh;
use game_model::textures::Texture;
use game_model::Node;

#[derive(Clone, Debug, Default)]
pub struct ModelIr {
    pub nodes: Vec<Node>,
    pub textures: Textures,
    pub materials: Materials,
    pub buffers: Buffers,
    pub meshes: Meshes,
}

#[derive(Clone, Debug, Default)]
pub struct Buffers {
    pub buffers: Vec<Buffer>,
}

impl Buffers {
    pub fn insert(&mut self, buffer: Buffer) -> Index {
        for (index, buf) in self.buffers.iter().enumerate() {
            if buf.bytes == buffer.bytes {
                return Index(index as u16);
            }
        }

        let index = self.buffers.len() as u16;
        if index == u16::MAX {
            panic!(
                "Buffer index overflow: cannot support more than {} distinct buffers",
                u16::MAX - 1
            );
        }

        self.buffers.push(buffer);
        Index(index)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Textures {
    pub textures: Vec<Texture>,
}

impl Textures {
    pub fn insert(&mut self, texture: Texture) -> Index {
        for (index, tex) in self.textures.iter().enumerate() {
            if texture_eq(&texture, tex) {
                return Index(index as u16);
            }
        }

        let index = self.textures.len() as u16;
        if index == u16::MAX {
            panic!(
                "Texture index overflow: cannot support more than {} distinct textures",
                u16::MAX - 1
            );
        }

        self.textures.push(texture);
        Index(index)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Index(pub u16);

#[derive(Clone, Debug, Default)]
pub struct Meshes {
    pub meshes: Vec<Mesh>,
}

impl Meshes {
    pub fn insert(&mut self, mesh: Mesh) -> Index {
        for (index, m) in self.meshes.iter().enumerate() {
            if mesh_eq(&mesh, m) {
                return Index(index as u16);
            }
        }

        let index = self.meshes.len() as u16;
        if index == u16::MAX {
            panic!(
                "Mesh index overflow: cannot support more than {} distinct meshes",
                u16::MAX - 1
            );
        }

        self.meshes.push(mesh);
        Index(index)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Materials {
    pub materials: Vec<Material>,
}

impl Materials {
    pub fn insert(&mut self, material: Material) -> Index {
        for (index, mat) in self.materials.iter().enumerate() {
            if material_eq(&material, mat) {
                return Index(index as u16);
            }
        }

        let index = self.materials.len() as u16;
        if index == u16::MAX {
            panic!(
                "Material index overflow: cannot support more than {} distinct materials",
                u16::MAX - 1
            );
        }

        self.materials.push(material);
        Index(index)
    }
}

/// Returns `true` if both textures are equal.
fn texture_eq(lhs: &Texture, rhs: &Texture) -> bool {
    if lhs.format != rhs.format || lhs.width != rhs.width || lhs.height != rhs.height {
        return false;
    }

    lhs.bytes == rhs.bytes
}

fn mesh_eq(lhs: &Mesh, rhs: &Mesh) -> bool {
    lhs.positions == rhs.positions
        && lhs.normals == rhs.normals
        && lhs.tangents == rhs.tangents
        && lhs.uvs == rhs.uvs
        && lhs.indices == rhs.indices
}

fn material_eq(lhs: &Material, rhs: &Material) -> bool {
    match (lhs, rhs) {
        (Material::MetallicRoughness(lhs), Material::MetallicRoughness(rhs)) => {
            lhs.base_color == rhs.base_color
                && lhs.roughness == rhs.roughness
                && lhs.metallic == rhs.metallic
                && lhs.albedo_texture == rhs.albedo_texture
                && lhs.normal_texture == rhs.normal_texture
                && lhs.metallic_roughness_texture == rhs.metallic_roughness_texture
        }
    }
}
