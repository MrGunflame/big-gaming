pub mod chunk;
pub mod terrain;

#[derive(Copy, Clone, Debug)]
pub struct ChunkId(i32);

impl ChunkId {
    pub fn from_coords(x: f32, z: f32) -> Self {
        let x = (x as i32) / 128;
        let y = (x as i32) / 128;

        Self(x << 16 | y)
    }

    pub fn to_coords(self) -> (f32, f32) {
        let x = self.0 >> 16;
        let z = self.0 as u32 & 0xFFFF_FFFFu32;

        (x as f32, z as f32)
    }
}

pub struct Chunk {}
