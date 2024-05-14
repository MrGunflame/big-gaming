use core::fmt::{self, Display, Formatter};

#[derive(Clone, Debug)]
pub struct ComponentDescriptor {
    fields: Box<[Field]>,
    root: Box<[FieldIndex]>,
}

impl ComponentDescriptor {
    pub fn new(fields: Vec<Field>, root: Vec<FieldIndex>) -> Result<Self, Error> {
        let len = fields.len();
        for field in &fields {
            match &field.kind {
                FieldKind::Int(_) | FieldKind::Float(_) => (),
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
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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

#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub kind: FieldKind,
}

#[derive(Clone, Debug)]
pub enum FieldKind {
    Int(IntegerField),
    Float(FloatField),
    Struct(Vec<FieldIndex>),
}

#[derive(Copy, Clone, Debug)]
pub struct IntegerField {
    pub bits: u8,
    pub is_signed: bool,
    pub min: Option<u64>,
    pub max: Option<u64>,
}

#[derive(Copy, Clone, Debug)]
pub struct FloatField {
    /// 32 or 64
    pub bits: u8,
}
