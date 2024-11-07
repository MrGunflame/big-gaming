//! Binary data format for the [`Prefab`] strucuture.

use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;

use bytes::{Buf, BufMut};
use game_wasm::world::RecordReference;

use crate::{ComponentRef, Prefab};

pub(crate) fn encode(prefab: &Prefab) -> Vec<u8> {
    let num_entities = prefab.entities.len() as u64;
    let num_children = prefab.children.len() as u64;
    let num_root = prefab.root.len() as u64;

    let mut buf = Vec::new();
    buf.put_u64_le(num_entities);
    buf.put_u64_le(num_children);
    buf.put_u64_le(num_root);

    for components in &prefab.entities {
        buf.put_u64_le(components.len() as u64);

        for data_ref in components {
            if cfg!(debug_assertions) {
                let _ = &prefab.data[data_ref.data.clone()];
                let _ = &prefab.data[data_ref.fields.clone()];
            }

            buf.put_slice(&data_ref.id.into_bytes());
            buf.put_u64_le(data_ref.data.start as u64);
            buf.put_u64_le(data_ref.data.len() as u64);
            buf.put_u64_le(data_ref.fields.start as u64);
            buf.put_u64_le(data_ref.fields.len() as u64);
        }
    }

    for (index, children) in &prefab.children {
        buf.put_u64_le(*index);
        buf.put_u64_le(children.len() as u64);

        for children in children {
            buf.put_u64_le(*children);
        }
    }

    for index in &prefab.root {
        buf.put_u64_le(*index);
    }

    buf.extend_from_slice(&prefab.data);

    buf
}

#[derive(Clone, Debug)]
pub enum DecodeError {
    Eof(EofError),
    InvalidEntityReference {
        index: usize,
        len: usize,
    },
    InvalidComponentReference {
        entity: usize,
        component: RecordReference,
        range: Range<usize>,
        data_len: usize,
    },
}

impl Display for DecodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eof(err) => Display::fmt(err, f),
            Self::InvalidEntityReference { index, len } => {
                write!(f, "invalid reference to entity {} (max is {})", index, len)
            }
            Self::InvalidComponentReference {
                entity,
                component,
                range,
                data_len,
            } => {
                write!(
                    f,
                    "component {} of entity {} refers to invalid data: {}..{} (len is {})",
                    component, entity, range.start, range.end, data_len
                )
            }
        }
    }
}

impl std::error::Error for DecodeError {}

#[derive(Clone, Debug)]
pub struct EofError {
    expected_len: usize,
    got_len: usize,
    section: Section,
}

impl Display for EofError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unexpected eof: expected {} bytes, got {} bytes decoding section {}",
            self.expected_len, self.got_len, self.section
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Section {
    Header,
    Root,
    Children,
    Entities,
}

impl Display for Section {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Header => write!(f, "header"),
            Self::Root => write!(f, "root"),
            Self::Children => write!(f, "children"),
            Self::Entities => write!(f, "entities"),
        }
    }
}

pub(crate) fn decode(mut buf: &[u8]) -> Result<Prefab, DecodeError> {
    if buf.remaining() < size_of::<u64>() * 3 {
        return Err(DecodeError::Eof(EofError {
            expected_len: size_of::<u64>() * 3,
            got_len: buf.remaining(),
            section: Section::Header,
        }));
    }

    let num_entities = buf.get_u64_le();
    let num_children = buf.get_u64_le();
    let num_root = buf.get_u64_le();

    let mut entities = Vec::new();
    let mut children = HashMap::new();
    let mut root = Vec::new();

    for _ in 0..num_entities {
        if buf.remaining() < size_of::<u64>() {
            return Err(DecodeError::Eof(EofError {
                expected_len: size_of::<u64>(),
                got_len: buf.remaining(),
                section: Section::Entities,
            }));
        }

        let len = buf.get_u64_le();

        let mut components = Vec::new();

        if buf.remaining() < 20 + size_of::<u64>() * 4 {
            return Err(DecodeError::Eof(EofError {
                expected_len: 20 + size_of::<u64>() * 4,
                got_len: buf.remaining(),
                section: Section::Entities,
            }));
        }

        for _ in 0..len {
            let mut id = [0; 20];
            buf.copy_to_slice(&mut id);

            let data_start = buf.get_u64_le();
            let data_len = buf.get_u64_le();
            let fields_start = buf.get_u64_le();
            let fields_len = buf.get_u64_le();

            components.push(ComponentRef {
                id: RecordReference::from_bytes(id),
                data: Range {
                    start: data_start as usize,
                    end: data_start as usize + data_len as usize,
                },
                fields: Range {
                    start: fields_start as usize,
                    end: fields_start as usize + fields_len as usize,
                },
            });
        }

        entities.push(components);
    }

    for _ in 0..num_children {
        if buf.remaining() < size_of::<u64>() * 2 {
            return Err(DecodeError::Eof(EofError {
                expected_len: size_of::<u64>() * 2,
                got_len: buf.remaining(),
                section: Section::Children,
            }));
        }

        let parent = buf.get_u64_le();
        let len = buf.get_u64_le();

        if buf.remaining() < size_of::<u64>() * len as usize {
            return Err(DecodeError::Eof(EofError {
                expected_len: size_of::<u64>() * 2,
                got_len: buf.remaining(),
                section: Section::Children,
            }));
        }

        let mut child_entities = Vec::new();
        for _ in 0..len {
            let index = buf.get_u64_le();

            if entities.get(index as usize).is_none() {
                return Err(DecodeError::InvalidEntityReference {
                    index: index as usize,
                    len: entities.len(),
                });
            }

            child_entities.push(index);
        }

        children.insert(parent, child_entities);
    }

    if buf.remaining() < size_of::<u64>() * num_root as usize {
        return Err(DecodeError::Eof(EofError {
            expected_len: size_of::<u64>() * num_root as usize,
            got_len: buf.remaining(),
            section: Section::Root,
        }));
    }

    for _ in 0..num_root {
        let index = buf.get_u64_le();

        if entities.get(index as usize).is_none() {
            return Err(DecodeError::InvalidEntityReference {
                index: index as usize,
                len: entities.len(),
            });
        }

        root.push(index);
    }

    let data = buf.to_vec();

    for (index, components) in entities.iter().enumerate() {
        for component in components {
            if data.get(component.data.clone()).is_none() {
                return Err(DecodeError::InvalidComponentReference {
                    entity: index,
                    component: component.id,
                    range: component.data.clone(),
                    data_len: data.len(),
                });
            }

            if data.get(component.fields.clone()).is_none() {
                return Err(DecodeError::InvalidComponentReference {
                    entity: index,
                    component: component.id,
                    range: component.fields.clone(),
                    data_len: data.len(),
                });
            }
        }
    }

    Ok(Prefab {
        entities,
        children,
        root,
        data,
    })
}

#[cfg(test)]
mod tests {
    use crate::Prefab;

    #[test]
    fn encode_and_decode_empty() {
        let prefab = Prefab::new();
        let buf = super::encode(&prefab);
        super::decode(&buf).unwrap();
    }
}
