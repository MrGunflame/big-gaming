use std::collections::HashMap;

use crate::entity::EntityId;

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
    /// A property that does not carry any value.
    ///
    /// `None` properties can be used as a marker property.
    None,
    I32(i32),
    I64(i64),
    Bytes(Box<[u8]>),
    Entity(EntityId),
}

impl PropertyValue {
    pub fn kind(&self) -> PropertyKind {
        match self {
            Self::None => PropertyKind::None,
            Self::I32(_) => PropertyKind::I32,
            Self::I64(_) => PropertyKind::I64,
            Self::Bytes(_) => PropertyKind::Bytes,
            Self::Entity(_) => PropertyKind::Entity,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PropertyKind {
    None,
    I32,
    I64,
    Bytes,
    Entity,
}
