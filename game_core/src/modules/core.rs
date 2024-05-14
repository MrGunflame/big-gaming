//! Builtin core module data

use game_common::components::{
    DirectionalLight, GlobalTransform, PointLight, SpotLight, Transform,
};
use game_common::record::ModuleId;
use game_common::reflection::{ComponentDescriptor, Field, FieldIndex, FieldKind, FloatField};
use game_data::components::components::ComponentRecord;
use game_data::record::{Record, RecordBody};
use game_wasm::components::Component;

use super::records::Records;
use super::ModuleData;

macro_rules! load_components {
    ($($t:ty),*$(,)?) => {
        {
            let mut data = ModuleData {
                id: ModuleId::CORE,
                records: Records::new(),
            };

            $(
                data.records.insert(Record {
                    id: <$t as Component>::ID.record,
                    name: stringify!($t).to_owned(),
                    components: Vec::new(),
                    scripts: Vec::new(),
                    body: RecordBody::Component(ComponentRecord {
                        description: "".to_owned(),
                        descriptor: <$t as Descriptor>::descriptor(),
                    })
                });
            )*

            data
        }
    };
}

pub fn load_core() -> ModuleData {
    load_components! {
        Transform,
        DirectionalLight,
        PointLight,
        SpotLight,
    }
}

trait Descriptor {
    fn descriptor() -> ComponentDescriptor;
}

impl Descriptor for Transform {
    fn descriptor() -> ComponentDescriptor {
        let fields = vec![
            Field {
                name: "X".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "Y".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "Z".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "W".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "Translation".to_owned(),
                kind: FieldKind::Struct(vec![
                    FieldIndex::from_raw(0),
                    FieldIndex::from_raw(1),
                    FieldIndex::from_raw(2),
                ]),
            },
            Field {
                name: "Rotation".to_owned(),
                kind: FieldKind::Struct(vec![
                    FieldIndex::from_raw(0),
                    FieldIndex::from_raw(1),
                    FieldIndex::from_raw(2),
                    FieldIndex::from_raw(3),
                ]),
            },
            Field {
                name: "Scale".to_owned(),
                kind: FieldKind::Struct(vec![
                    FieldIndex::from_raw(0),
                    FieldIndex::from_raw(1),
                    FieldIndex::from_raw(2),
                ]),
            },
        ];
        let root = vec![
            FieldIndex::from_raw(4),
            FieldIndex::from_raw(5),
            FieldIndex::from_raw(6),
        ];

        ComponentDescriptor::new(fields, root).unwrap()
    }
}

impl Descriptor for GlobalTransform {
    fn descriptor() -> ComponentDescriptor {
        Transform::descriptor()
    }
}

impl Descriptor for DirectionalLight {
    fn descriptor() -> ComponentDescriptor {
        let fields = vec![
            Field {
                name: "R".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "G".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "B".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "A".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "Color".to_owned(),
                kind: FieldKind::Struct(vec![
                    FieldIndex::from_raw(0),
                    FieldIndex::from_raw(1),
                    FieldIndex::from_raw(2),
                    FieldIndex::from_raw(3),
                ]),
            },
            Field {
                name: "Illuminance".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
        ];
        let root = vec![FieldIndex::from_raw(4), FieldIndex::from_raw(5)];

        ComponentDescriptor::new(fields, root).unwrap()
    }
}

impl Descriptor for PointLight {
    fn descriptor() -> ComponentDescriptor {
        let fields = vec![
            Field {
                name: "R".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "G".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "B".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "A".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "Color".to_owned(),
                kind: FieldKind::Struct(vec![
                    FieldIndex::from_raw(0),
                    FieldIndex::from_raw(1),
                    FieldIndex::from_raw(2),
                    FieldIndex::from_raw(3),
                ]),
            },
            Field {
                name: "Intensity".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "Radius".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
        ];
        let root = vec![
            FieldIndex::from_raw(4),
            FieldIndex::from_raw(5),
            FieldIndex::from_raw(6),
        ];

        ComponentDescriptor::new(fields, root).unwrap()
    }
}

impl Descriptor for SpotLight {
    fn descriptor() -> ComponentDescriptor {
        let fields = vec![
            Field {
                name: "R".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "G".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "B".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "A".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "Color".to_owned(),
                kind: FieldKind::Struct(vec![
                    FieldIndex::from_raw(0),
                    FieldIndex::from_raw(1),
                    FieldIndex::from_raw(2),
                    FieldIndex::from_raw(3),
                ]),
            },
            Field {
                name: "Intensity".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "Radius".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "Inner Cutoff".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "Outer Cutoff".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
        ];
        let root = vec![
            FieldIndex::from_raw(4),
            FieldIndex::from_raw(5),
            FieldIndex::from_raw(6),
            FieldIndex::from_raw(7),
            FieldIndex::from_raw(8),
        ];

        ComponentDescriptor::new(fields, root).unwrap()
    }
}
