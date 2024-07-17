use bytes::{Buf, BufMut};
use game_common::module::{Dependency, Module, ModuleId, PreRelease, Version};
use thiserror::Error;

use crate::varint::VarU64;
use crate::{Decode, Encode, EofError};

pub const MAGIC: [u8; 4] = [0, 0, 0, 0];

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum HeaderError {
    #[error("failed to read header magic: {0}")]
    Magic(<[u8; 4] as Decode>::Error),
    #[error("invalid magic: {0:?}")]
    InvalidMagic([u8; 4]),
    #[error("failed to read header version: {0}")]
    Version(<u8 as Decode>::Error),
    #[error("failed to read module header: {0}")]
    Module(<Module as Decode>::Error),
}

#[derive(Clone, Debug)]
pub struct Header {
    // magic outlined
    pub version: u8,

    pub module: Module,
}

impl Encode for Header {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        MAGIC.encode(&mut buf);

        self.version.encode(&mut buf);
        self.module.encode(&mut buf);
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

        Ok(Self { version, module })
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

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ModuleError {
    #[error("failed to decode id: {0}")]
    Id(<ModuleId as Decode>::Error),
    #[error("failed to decode name: {0}")]
    Name(<String as Decode>::Error),
    #[error("failed to decode version: {0}")]
    Version(<Version as Decode>::Error),
    #[error("failed to decode dependencies: {0}")]
    Dependencies(<Vec<Dependency> as Decode>::Error),
}

impl Encode for Module {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.id.encode(&mut buf);
        self.name.encode(&mut buf);
        self.version.encode(&mut buf);
        self.dependencies.encode(&mut buf);
    }
}

impl Decode for Module {
    type Error = ModuleError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = ModuleId::decode(&mut buf).map_err(ModuleError::Id)?;
        let name = String::decode(&mut buf).map_err(ModuleError::Name)?;
        let version = Version::decode(&mut buf).map_err(ModuleError::Version)?;
        let dependencies = Vec::decode(&mut buf).map_err(ModuleError::Dependencies)?;

        Ok(Self {
            id,
            name,
            version,
            dependencies,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum DependencyError {
    #[error("failed to decode id: {0}")]
    Id(<ModuleId as Decode>::Error),
    #[error("failed to decode name: {0}")]
    Name(<String as Decode>::Error),
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
    type Error = DependencyError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = ModuleId::decode(&mut buf).map_err(DependencyError::Id)?;
        let name = String::decode(&mut buf).map_err(DependencyError::Name)?;

        Ok(Self {
            id,
            name: Some(name),
            // TODO: Provide a dependency requirement here.
            version: Version::new(0, 0, 0),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum VersionError {
    #[error("bad major: {0}")]
    Major(<VarU64 as Decode>::Error),
    #[error("bad minor: {0}")]
    Minor(<VarU64 as Decode>::Error),
    #[error("bad patch: {0}")]
    Patch(<VarU64 as Decode>::Error),
    #[error("bad pre-release: {0}")]
    PreRelease(<String as Decode>::Error),
}

impl Encode for Version {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        VarU64(self.major).encode(&mut buf);
        VarU64(self.minor).encode(&mut buf);
        VarU64(self.patch).encode(&mut buf);
        self.pre_release.as_str().encode(&mut buf);
    }
}

impl Decode for Version {
    type Error = VersionError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let VarU64(major) = VarU64::decode(&mut buf).map_err(VersionError::Major)?;
        let VarU64(minor) = VarU64::decode(&mut buf).map_err(VersionError::Minor)?;
        let VarU64(patch) = VarU64::decode(&mut buf).map_err(VersionError::Patch)?;
        let pre_release = String::decode(&mut buf).map_err(VersionError::PreRelease)?;

        Ok(Self {
            major,
            minor,
            patch,
            pre_release: PreRelease::new(&pre_release),
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
            id: ModuleId::CORE,
            name: String::from("test"),
            version: Version::new(1, 0, 0),
            dependencies: vec![Dependency {
                id: ModuleId::CORE,
                name: Some(String::from("dep")),
                version: Version::new(0, 0, 0),
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
