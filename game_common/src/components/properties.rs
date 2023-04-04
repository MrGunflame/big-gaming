use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Properties {
    props: Vec<Property>,
}

impl Properties {
    pub fn new() -> Self {
        Self { props: Vec::new() }
    }

    pub fn insert(&mut self, property: Property) {
        self.props.push(property);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PropertyId(u32);

#[derive(Clone, Debug)]
pub struct Property {
    pub name: String,
    pub value: PropertyValue,
}

#[derive(Clone, Debug)]
pub enum PropertyValue {
    I32(i32),
    I64(i64),
    Bytes(Box<[u8]>),
}

impl PropertyValue {
    pub fn kind(&self) -> PropertyKind {
        match self {
            Self::I32(_) => PropertyKind::I32,
            Self::I64(_) => PropertyKind::I64,
            Self::Bytes(_) => PropertyKind::Bytes,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PropertyKind {
    I32,
    I64,
    Bytes,
}
