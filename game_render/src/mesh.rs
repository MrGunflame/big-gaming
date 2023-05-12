use wgpu::PrimitiveTopology;

#[derive(Clone, Debug)]
pub struct Mesh {
    topology: PrimitiveTopology,
    indices: Option<Indices>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            topology: PrimitiveTopology::TriangleList,
            indices: None,
        }
    }

    pub fn set_indices(&mut self, indices: Indices) {
        self.indices = Some(indices);
    }
}

#[derive(Clone, Debug)]
pub enum Indices {
    U16(Vec<u16>),
    U32(Vec<u32>),
}
