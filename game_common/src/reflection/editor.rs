use std::collections::VecDeque;

use crate::components::components::RawComponent;

use super::{ComponentDescriptor, FieldIndex, FieldKind};

#[derive(Clone, Debug)]
pub struct ComponentEditor<'a> {
    descriptor: &'a ComponentDescriptor,
    component: RawComponent,
}

impl<'a> ComponentEditor<'a> {
    pub fn new(descriptor: &'a ComponentDescriptor, component: RawComponent) -> Self {
        Self {
            descriptor,
            component,
        }
    }

    pub fn descriptor(&self) -> &'a ComponentDescriptor {
        self.descriptor
    }

    pub fn get(&self, field: FieldIndex) -> Option<&[u8]> {
        let offset = self.get_field_offset(field)?;

        let field = self.descriptor.get(field)?;
        let len = match field.kind {
            FieldKind::Int(field) => usize::from(field.bits) / 8,
            FieldKind::Float(field) => usize::from(field.bits) / 8,
            _ => todo!(),
        };

        Some(&self.component.as_bytes()[offset..offset + len])
    }

    fn get_field_offset(&self, dst: FieldIndex) -> Option<usize> {
        if usize::from(dst.0) > self.descriptor.fields.len() {
            return None;
        }

        let mut fields: VecDeque<FieldIndex> = VecDeque::new();
        fields.extend(self.descriptor.root());

        let mut offset = 0;
        while let Some(index) = fields.pop_front() {
            if index == dst {
                return Some(offset);
            }

            let field = self.descriptor.get(index)?;
            match &field.kind {
                FieldKind::Int(field) => {
                    let len = usize::from(field.bits) / 8;
                    offset += len;
                }
                FieldKind::Float(field) => {
                    let len = usize::from(field.bits) / 8;
                    offset += len;
                }
                FieldKind::Struct(field) => {
                    for index in field {
                        fields.push_front(*index);
                    }
                }
                FieldKind::Enum(_) => todo!(),
                FieldKind::String => todo!(),
            }
        }

        None
    }
}
