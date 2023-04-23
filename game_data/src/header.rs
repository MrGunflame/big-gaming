use bytes::{Buf, BufMut};
use game_common::module::{Dependency, Module, ModuleId, Version};
use thiserror::Error;

use crate::{Decode, Encode, EofError};

pub const MAGIC: [u8; 4] = [0, 0, 0, 0];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum HeaderError {
    #[error("failed to read header magic: {0}")]
    Magic(<[u8; 4] as Decode>::Error),
    #[error("invalid magic: {0:?}")]
    InvalidMagic([u8; 4]),
    #[error("failed to read header version: {0}")]
    Version(<u8 as Decode>::Error),
    #[error("failed to read module header: {0}")]
    Module(<Module as Decode>::Error),
    #[error("failed to read item count: {0}")]
    Items(<u32 as Decode>::Error),
    #[error("failed to read patch count: {0}")]
    Patches(<u32 as Decode>::Error),
}

#[derive(Clone, Debug)]
pub struct Header {
    // magic outlined
    pub version: u8,

    pub module: Module,

    pub items: u32,
    pub patches: u32,
}

impl Encode for Header {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        MAGIC.encode(&mut buf);

        self.version.encode(&mut buf);
        self.module.encode(&mut buf);
        self.items.encode(&mut buf);
        self.patches.encode(&mut buf);
    }
}

impl Decode for Header {
    type Error = HeaderError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let magic = <[u8; 4]>::decode(&mut buf).map_err(HeaderError::Magic)?;

        if magic != MAGIC {
            return Err(HeaderError::InvalidMagic(magic));
        }

        let version = u8::decode(&mut buf).map_err(HeaderError::Version)?;
        let module = Module::decode(&mut buf).map_err(HeaderError::Module)?;
        let items = u32::decode(&mut buf).map_err(HeaderError::Items)?;
        let patches = u32::decode(&mut buf).map_err(HeaderError::Patches)?;

        Ok(Self {
            version,
            items,
            module,
            patches,
        })
    }
}

impl Encode for ModuleId {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.put_slice(&self.into_bytes());
    }
}

impl Decode for ModuleId {
    type Error = EofError;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let bytes = <[u8; 16]>::decode(buf).map_err(|err| EofError {
            on: "ModuleId",
            consumed: err.consumed,
            expected: err.expected,
        })?;

        Ok(Self::from_bytes(bytes))
    }
}

impl Encode for Module {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.id.encode(&mut buf);
        self.name.encode(&mut buf);
        // self.version

        (self.dependencies.len() as u32).encode(&mut buf);
        for dep in &self.dependencies {
            dep.encode(&mut buf);
        }
    }
}

impl Decode for Module {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = ModuleId::decode(&mut buf)?;
        let name = String::decode(&mut buf)?;

        let len = u32::decode(&mut buf)?;
        let mut dependencies = Vec::new();

        for _ in 0..len {
            let dependency = Dependency::decode(&mut buf)?;
            dependencies.push(dependency);
        }

        Ok(Self {
            id,
            name,
            version: Version,
            dependencies,
        })
    }
}

impl Encode for Dependency {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.id.encode(&mut buf);
        let name = self.name.as_ref().map(|s| s.as_str()).unwrap_or("<empty>");
        name.encode(&mut buf);

        // self.version
    }
}

impl Decode for Dependency {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = ModuleId::decode(&mut buf)?;
        let name = String::decode(&mut buf)?;

        Ok(Self {
            id,
            name: Some(name),
            version: Version,
        })
    }
}

#[cfg(test)]
mod tests {
    use game_common::module::{Dependency, Module, ModuleId, Version};

    use crate::{Decode, Encode};

    #[test]
    fn test_module_reflexive() {
        let module = Module {
            id: ModuleId::random(),
            name: String::from("test"),
            version: Version,
            dependencies: vec![Dependency {
                id: ModuleId::random(),
                name: Some(String::from("dep")),
                version: Version,
            }],
        };

        let mut buf = Vec::new();
        module.encode(&mut buf);

        let res = Module::decode(&buf[..]).unwrap();

        assert_eq!(module.id, res.id);
        assert_eq!(module.name, res.name);
        assert_eq!(module.version, res.version);

        assert_eq!(module.dependencies.len(), res.dependencies.len());
        for (d1, d2) in module.dependencies.iter().zip(res.dependencies) {
            assert_eq!(d1.id, d2.id);
            assert_eq!(d1.name, d2.name);
            assert_eq!(d1.version, d2.version);
        }
    }
}
