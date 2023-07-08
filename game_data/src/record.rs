use bytes::{Buf, BufMut};
use game_common::module::ModuleId;
use game_common::record::{RecordId, RecordReference};
use thiserror::Error;

use crate::components::actions::ActionRecord;
use crate::components::components::ComponentRecord;
use crate::components::item::ItemRecord;
use crate::components::objects::ObjectRecord;
use crate::uri::Uri;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum RecordReferenceError {
    #[error("failed to decode module ref: {0}")]
    Module(<ModuleId as Decode>::Error),
    #[error("failed to decode record ref: {0}")]
    Record(<RecordId as Decode>::Error),
}

// impl Display for RecordReference {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         write!(f, "{}:{}", self.module, self.record)
//     }
// }

impl Encode for RecordReference {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.module.encode(&mut buf);
        self.record.encode(&mut buf);
    }
}

impl Decode for RecordReference {
    type Error = RecordReferenceError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let module = ModuleId::decode(&mut buf).map_err(RecordReferenceError::Module)?;
        let record = RecordId::decode(&mut buf).map_err(RecordReferenceError::Record)?;

        Ok(Self { module, record })
    }
}

// impl Display for RecordId {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         LowerHex::fmt(&self.0, f)
//     }
// }

#[derive(Clone, Debug)]
pub struct Record {
    pub id: RecordId,
    pub name: String,
    pub scripts: Vec<Uri>,
    pub body: RecordBody,
}

impl Record {
    pub const fn kind(&self) -> RecordKind {
        self.body.kind()
    }
}

#[derive(Clone, Debug)]
pub enum RecordBody {
    Item(ItemRecord),
    Action(ActionRecord),
    Component(ComponentRecord),
    Object(ObjectRecord),
}

impl RecordBody {
    pub const fn kind(&self) -> RecordKind {
        match self {
            Self::Item(_) => RecordKind::Item,
            Self::Action(_) => RecordKind::Action,
            Self::Component(_) => RecordKind::Component,
            Self::Object(_) => RecordKind::Object,
        }
    }

    #[deprecated]
    pub fn unwrap_item(self) -> ItemRecord {
        match self {
            Self::Item(item) => item,
            _ => self.panic_invalid_record("item"),
        }
    }

    #[deprecated]
    pub fn unwrap_action(self) -> ActionRecord {
        match self {
            Self::Action(action) => action,
            _ => self.panic_invalid_record("action"),
        }
    }

    #[deprecated]
    pub fn unwrap_componen(self) -> ComponentRecord {
        match self {
            Self::Component(component) => component,
            _ => self.panic_invalid_record("component"),
        }
    }

    #[deprecated]
    pub fn unwrap_object(self) -> ObjectRecord {
        match self {
            Self::Object(object) => object,
            _ => self.panic_invalid_record("object"),
        }
    }

    pub fn as_item(&self) -> Option<&ItemRecord> {
        match self {
            Self::Item(item) => Some(item),
            _ => None,
        }
    }

    pub fn as_action(&self) -> Option<&ActionRecord> {
        match self {
            Self::Action(action) => Some(action),
            _ => None,
        }
    }

    pub fn as_component(&self) -> Option<&ComponentRecord> {
        match self {
            Self::Component(component) => Some(component),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&ObjectRecord> {
        match self {
            Self::Object(object) => Some(object),
            _ => None,
        }
    }

    pub fn into_item(self) -> Option<ItemRecord> {
        match self {
            Self::Item(item) => Some(item),
            _ => None,
        }
    }

    pub fn into_action(self) -> Option<ActionRecord> {
        match self {
            Self::Action(action) => Some(action),
            _ => None,
        }
    }

    pub fn into_component(self) -> Option<ComponentRecord> {
        match self {
            Self::Component(component) => Some(component),
            _ => None,
        }
    }

    pub fn into_object(self) -> Option<ObjectRecord> {
        match self {
            Self::Object(object) => Some(object),
            _ => None,
        }
    }

    #[inline(never)]
    #[cold]
    fn panic_invalid_record(&self, expected: &'static str) -> ! {
        let got = match self.kind() {
            RecordKind::Item => "Item",
            RecordKind::Action => "Action",
            RecordKind::Component => "Component",
            RecordKind::Object => "Object",
        };

        panic!(
            "called `RecordBody::unwrap_{}` on a {} value",
            expected, got
        );
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RecordKind {
    Item,
    Action,
    Component,
    Object,
}

impl RecordKind {
    pub const fn is_action(&self) -> bool {
        matches!(self, Self::Action)
    }

    pub const fn is_component(&self) -> bool {
        matches!(self, Self::Component)
    }

    pub const fn is_item(&self) -> bool {
        matches!(self, Self::Item)
    }

    pub const fn is_object(&self) -> bool {
        matches!(self, Self::Object)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum RecordKindError {
    #[error("failed to decode record kind byte: {0}")]
    Byte(<u8 as Decode>::Error),
    #[error("found invalid record kind: {0}")]
    InvalidKind(u8),
}

impl Encode for RecordKind {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        let byte: u8 = match self {
            Self::Item => 1,
            Self::Action => 2,
            Self::Component => 3,
            Self::Object => 4,
        };

        byte.encode(buf);
    }
}

impl Decode for RecordKind {
    type Error = RecordKindError;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let byte = u8::decode(buf).map_err(RecordKindError::Byte)?;

        match byte {
            1 => Ok(Self::Item),
            2 => Ok(Self::Action),
            3 => Ok(Self::Component),
            4 => Ok(Self::Object),
            _ => Err(RecordKindError::InvalidKind(byte)),
        }
    }
}

impl Encode for RecordId {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        self.0.encode(buf);
    }
}

impl Decode for RecordId {
    type Error = <u32 as Decode>::Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u32::decode(buf).map(Self)
    }
}

impl Encode for Record {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        self.id.encode(&mut buf);
        self.name.encode(&mut buf);
        self.scripts.encode(&mut buf);

        self.body.kind().encode(&mut buf);
        match &self.body {
            RecordBody::Item(item) => {
                item.encode(&mut buf);
            }
            RecordBody::Action(action) => {
                action.encode(&mut buf);
            }
            RecordBody::Component(component) => {
                component.encode(&mut buf);
            }
            RecordBody::Object(object) => {
                object.encode(&mut buf);
            }
        };
    }
}

impl Decode for Record {
    type Error = RecordError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = RecordId::decode(&mut buf).map_err(RecordError::Id)?;
        let name = String::decode(&mut buf).map_err(RecordError::Name)?;
        let scripts = Vec::decode(&mut buf).map_err(RecordError::Scripts)?;
        let kind = RecordKind::decode(&mut buf).map_err(RecordError::Kind)?;

        let body = match kind {
            RecordKind::Item => {
                let item = ItemRecord::decode(&mut buf)?;
                RecordBody::Item(item)
            }
            RecordKind::Action => {
                let action = ActionRecord::decode(&mut buf)?;
                RecordBody::Action(action)
            }
            RecordKind::Component => {
                let component = ComponentRecord::decode(&mut buf)?;
                RecordBody::Component(component)
            }
            RecordKind::Object => {
                let object = ObjectRecord::decode(&mut buf)?;
                RecordBody::Object(object)
            }
        };

        Ok(Self {
            id,
            name,
            scripts,
            body,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum RecordError {
    #[error("failed to decode record id: {0}")]
    Id(<RecordId as Decode>::Error),
    #[error("failed to decode record name: {0}")]
    Name(<String as Decode>::Error),
    #[error("failed to decode scripts: {0}")]
    Scripts(<Vec<Uri> as Decode>::Error),
    #[error("failed to decode record kind: {0}")]
    Kind(<RecordKind as Decode>::Error),
    #[error("failed to decode item record: {0}")]
    Item(#[from] <ItemRecord as Decode>::Error),
    #[error("failed to decode action record: {0}")]
    Action(#[from] <ActionRecord as Decode>::Error),
    #[error("failed to decode component record: {0}")]
    Component(#[from] <ComponentRecord as Decode>::Error),
    #[error("failed to decode object record: {0}")]
    Object(#[from] <ObjectRecord as Decode>::Error),
}
