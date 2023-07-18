use std::convert::Infallible;

use bytes::{Buf, BufMut};
use game_common::world::entity::Terrain;
use game_common::world::terrain::{Heightmap, TerrainMesh};
use game_common::world::CellId;
use glam::UVec2;

use super::{Decode, Encode, EofError};

impl Encode for Terrain {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.mesh.cell.encode(&mut buf)?;
        self.mesh.height().size().encode(&mut buf)?;

        for texel in self.mesh.offsets.as_u8() {
            texel.encode(&mut buf)?;
        }

        Ok(())
    }
}

impl Decode for Terrain {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let cell = CellId::decode(&mut buf)?;
        let size = UVec2::decode(&mut buf)?;

        // FIXME: This may overflow on 32-bit if x and y are too big.
        // While this isn't a problem on 64-bit platforms it still isn't
        // optimal as we probably don't want a chunk that big. We can
        // instead lower the size of `size`, making this potentail overflow
        // impossible and reducing the protocol overhead.
        let len = (size.x as usize).checked_mul(size.y as usize).unwrap();

        let mut texels = Vec::new();
        for _ in 0..len {
            let texel = u8::decode(&mut buf)?;
            texels.push(texel);
        }

        Ok(Self {
            mesh: TerrainMesh {
                cell,
                offsets: Heightmap::from_u8(size, texels),
            },
        })
    }
}
