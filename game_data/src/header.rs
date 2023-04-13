use bytes::{Buf, BufMut};
use game_common::module::{Dependency, Module, ModuleId, Version};

use crate::{Decode, Encode, EofError};

pub const MAGIC: [u8; 4] = [0, 0, 0, 0];

#[derive(Clone, Debug)]
pub struct Header {
    // magic outlined
    pub version: u8,

    pub module: Module,

    pub items: u32,
}

impl Encode for Header {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.put_slice(&MAGIC);

        self.version.encode(&mut buf);
        self.module.encode(&mut buf);
        self.items.encode(&mut buf);
    }
}

impl Decode for Header {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let magic = u32::decode(&mut buf)?;
        assert!(magic == u32::from_ne_bytes(MAGIC));

        let version = u8::decode(&mut buf)?;
        let module = Module::decode(&mut buf)?;
        let items = u32::decode(&mut buf)?;

        Ok(Self {
            version,
            items,
            module,
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

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mut bytes = [0; 16];
        buf.copy_to_slice(&mut bytes);
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
