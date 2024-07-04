use core::fmt::{self, Display, Formatter};
use std::collections::VecDeque;

use game_wasm::encoding::Primitive;
use serde::{Deserialize, Serialize};

use crate::components::components::RawComponent;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComponentDescriptor {
    fields: Box<[Field]>,
    root: Box<[FieldIndex]>,
}

impl ComponentDescriptor {
    pub fn new(fields: Vec<Field>, root: Vec<FieldIndex>) -> Result<Self, Error> {
        let len = fields.len();
        for field in &fields {
            match &field.kind {
                FieldKind::Int(_) | FieldKind::Float(_) | FieldKind::String(_) => (),
                FieldKind::Struct(indices) => {
                    for index in indices {
                        if usize::from(index.0) >= len {
                            return Err(Error::InvalidFieldIndex {
                                index: *index,
                                field: field.name.clone(),
                            });
                        }
                    }
                }
            }
        }

        for index in &root {
            if usize::from(index.0) >= len {
                return Err(Error::InvalidRootFieldIndex { index: *index });
            }
        }

        Ok(Self {
            fields: fields.into_boxed_slice(),
            root: root.into_boxed_slice(),
        })
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    pub fn root(&self) -> &[FieldIndex] {
        &self.root
    }

    pub fn get(&self, index: FieldIndex) -> Option<&Field> {
        self.fields.get(usize::from(index.0))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
    }

    /// Returns a new, default [`RawComponent`] for the `ComponentDescriptor`.
    pub fn default_component(&self) -> RawComponent {
        let mut bytes = Vec::new();
        let mut fields = Vec::new();
        let mut offset = 0;

        let mut queue: VecDeque<FieldIndex> = VecDeque::new();
        queue.extend(self.root.iter());

        while let Some(index) = queue.pop_front() {
            let field = &self.fields[usize::from(index.0)];

            match &field.kind {
                FieldKind::Int(field) => {
                    let mut num_bytes = usize::from(field.bits) / 8;
                    if field.bits % 8 != 0 {
                        num_bytes += 1;
                    }

                    bytes.resize(bytes.len() + num_bytes, 0);
                    fields.push(game_wasm::encoding::Field {
                        primitive: Primitive::Bytes,
                        offset,
                    });
                    offset += num_bytes;
                }
                FieldKind::Float(field) => match field.bits {
                    32 => {
                        bytes.extend(0f32.to_le_bytes());
                        fields.push(game_wasm::encoding::Field {
                            primitive: Primitive::Bytes,
                            offset,
                        });
                        offset += 4;
                    }
                    64 => {
                        bytes.extend(0f64.to_le_bytes());
                        fields.push(game_wasm::encoding::Field {
                            primitive: Primitive::Bytes,
                            offset,
                        });
                        offset += 4;
                    }
                    _ => todo!(),
                },
                FieldKind::Struct(field) => {
                    for index in field.iter().rev() {
                        queue.push_front(*index);
                    }
                }
                FieldKind::String(_) => {
                    bytes.extend(0u64.to_le_bytes());
                }
            }
        }

        RawComponent::new(bytes, fields)
    }
}

impl Default for ComponentDescriptor {
    fn default() -> Self {
        Self {
            fields: Box::new([]),
            root: Box::new([]),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FieldIndex(u16);

impl FieldIndex {
    #[inline]
    pub const fn from_raw(bits: u16) -> Self {
        Self(bits)
    }

    #[inline]
    pub const fn into_raw(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Error {
    InvalidFieldIndex { index: FieldIndex, field: String },
    InvalidRootFieldIndex { index: FieldIndex },
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFieldIndex { index, field } => {
                write!(f, "invalid field index {} at {}", index.into_raw(), field)
            }
            Self::InvalidRootFieldIndex { index } => {
                write!(f, "invalid root field index {}", index.into_raw())
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub kind: FieldKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FieldKind {
    Int(IntegerField),
    Float(FloatField),
    Struct(Vec<FieldIndex>),
    String(String),
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct IntegerField {
    pub bits: u8,
    pub is_signed: bool,
    pub min: Option<u64>,
    pub max: Option<u64>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct FloatField {
    /// 32 or 64
    pub bits: u8,
}
