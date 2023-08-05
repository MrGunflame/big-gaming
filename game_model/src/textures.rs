use bytes::{Buf, BufMut};

use crate::{Decode, Encode};

#[derive(Clone, Debug)]
pub struct Texture {
    pub format: TextureFormat,
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

impl Encode for Texture {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        debug_assert_eq!(
            self.width as usize * self.height as usize * self.format.pixel_size() as usize,
            self.bytes.len(),
        );

        self.format.encode(&mut buf);
        self.width.encode(&mut buf);
        self.height.encode(&mut buf);

        for b in &self.bytes {
            b.encode(&mut buf);
        }
    }
}

impl Decode for Texture {
    type Error = ();

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let format = TextureFormat::decode(&mut buf)?;
        let width = u32::decode(&mut buf)?;
        let height = u32::decode(&mut buf)?;

        let size = width as usize * height as usize * format.pixel_size() as usize;
        let mut bytes = Vec::new();
        for _ in 0..size {
            bytes.push(u8::decode(&mut buf)?);
        }

        Ok(Self {
            format,
            width,
            height,
            bytes,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    Rgba8UnormSrgb,
    Rgba8Unorm,
}

impl TextureFormat {
    fn pixel_size(self) -> u32 {
        match self {
            Self::Rgba8UnormSrgb => 4,
            Self::Rgba8Unorm => 4,
        }
    }
}

impl Encode for TextureFormat {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        let b: u8 = match self {
            Self::Rgba8UnormSrgb => 1,
            Self::Rgba8Unorm => 2,
        };

        b.encode(buf);
    }
}

impl Decode for TextureFormat {
    type Error = ();

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let b = u8::decode(buf)?;

        match b {
            1 => Ok(Self::Rgba8UnormSrgb),
            2 => Ok(Self::Rgba8Unorm),
            _ => Err(()),
        }
    }
}
