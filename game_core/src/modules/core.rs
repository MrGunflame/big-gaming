//! Builtin core module data

use game_common::components::{
    Collider, DirectionalLight, GlobalTransform, MeshInstance, PointLight, RigidBody, SpotLight,
    Transform,
};
use game_common::record::ModuleId;
use game_common::reflection::{
    ComponentDescriptor, EnumField, EnumFieldVariant, Field, FieldIndex, FieldKind, FloatField,
    RecordDescriptor,
};
use game_data::record::{Record, RecordKind};
use game_wasm::components::Component;
use game_wasm::record::RecordId;
use game_wasm::world::RecordReference;

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
                    kind: RecordKind::COMPONENT,
                    description: String::new(),
                    data: <$t as Descriptor>::descriptor().to_bytes(),
                });
            )*

            data
        }
    };
}

pub fn load_core() -> ModuleData {
    let mut data = load_components! {
        Transform,
        DirectionalLight,
        PointLight,
        SpotLight,
        Collider,
        RigidBody,
        MeshInstance,
    };

    load_core_records(&mut data);
    data
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

impl Descriptor for Collider {
    fn descriptor() -> ComponentDescriptor {
        let fields = vec![
            // Cuboid
            Field {
                name: "hx".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "hy".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "hz".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            // Ball
            Field {
                name: "Radius".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            // Capsule
            Field {
                name: "Axis".to_owned(),
                kind: FieldKind::Enum(EnumField {
                    tag_bits: 8,
                    default_variant: 0,
                    variants: vec![
                        EnumFieldVariant {
                            tag: 0,
                            name: "X".to_owned(),
                            fields: Vec::new(),
                        },
                        EnumFieldVariant {
                            tag: 1,
                            name: "Y".to_owned(),
                            fields: Vec::new(),
                        },
                        EnumFieldVariant {
                            tag: 2,
                            name: "Z".to_owned(),
                            fields: Vec::new(),
                        },
                    ],
                }),
            },
            Field {
                name: "Half height".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "Radius".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "Friction".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "Restitution".to_owned(),
                kind: FieldKind::Float(FloatField { bits: 32 }),
            },
            Field {
                name: "Shape".to_owned(),
                kind: FieldKind::Enum(EnumField {
                    tag_bits: 8,
                    default_variant: 1,
                    variants: vec![
                        EnumFieldVariant {
                            tag: 1,
                            name: "Cuboid".to_owned(),
                            fields: vec![
                                FieldIndex::from_raw(0),
                                FieldIndex::from_raw(1),
                                FieldIndex::from_raw(2),
                            ],
                        },
                        EnumFieldVariant {
                            tag: 2,
                            name: "Ball".to_owned(),
                            fields: vec![FieldIndex::from_raw(3)],
                        },
                        EnumFieldVariant {
                            tag: 3,
                            name: "Capsulse".to_owned(),
                            fields: vec![
                                FieldIndex::from_raw(4),
                                FieldIndex::from_raw(5),
                                FieldIndex::from_raw(6),
                            ],
                        },
                    ],
                }),
            },
        ];
        let root = vec![
            FieldIndex::from_raw(7),
            FieldIndex::from_raw(8),
            FieldIndex::from_raw(9),
        ];

        ComponentDescriptor::new(fields, root).unwrap()
    }
}

impl Descriptor for RigidBody {
    fn descriptor() -> ComponentDescriptor {
        let fields = vec![
            // Vec3
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
                name: "Kind".to_owned(),
                kind: FieldKind::Enum(EnumField {
                    tag_bits: 8,
                    default_variant: 0,
                    variants: vec![
                        EnumFieldVariant {
                            tag: 0,
                            name: "Fixed".to_owned(),
                            fields: Vec::new(),
                        },
                        EnumFieldVariant {
                            tag: 1,
                            name: "Dynamic".to_owned(),
                            fields: Vec::new(),
                        },
                        EnumFieldVariant {
                            tag: 2,
                            name: "Kinematic".to_owned(),
                            fields: Vec::new(),
                        },
                    ],
                }),
            },
            Field {
                name: "Linear Velocity".to_owned(),
                kind: FieldKind::Struct(vec![
                    FieldIndex::from_raw(0),
                    FieldIndex::from_raw(1),
                    FieldIndex::from_raw(2),
                ]),
            },
            Field {
                name: "Angular Velocity".to_owned(),
                kind: FieldKind::Struct(vec![
                    FieldIndex::from_raw(0),
                    FieldIndex::from_raw(1),
                    FieldIndex::from_raw(2),
                ]),
            },
        ];
        let root = vec![
            FieldIndex::from_raw(3),
            FieldIndex::from_raw(4),
            FieldIndex::from_raw(5),
        ];

        ComponentDescriptor::new(fields, root).unwrap()
    }
}

impl Descriptor for MeshInstance {
    fn descriptor() -> ComponentDescriptor {
        let fields = vec![Field {
            name: "Path".to_owned(),
            kind: FieldKind::String,
        }];
        let root = vec![FieldIndex::from_raw(0)];

        ComponentDescriptor::new(fields, root).unwrap()
    }
}

fn load_core_records(data: &mut ModuleData) {
    for (id, name, descriptor) in [
        (
            RecordKind::COMPONENT.0.record,
            "Component",
            RecordDescriptor {
                component: RecordReference::STUB,
                keys: Vec::new(),
            },
        ),
        (
            RecordKind::RECORD.0.record,
            "Record",
            RecordDescriptor {
                component: RecordReference::STUB,
                keys: Vec::new(),
            },
        ),
        (
            RecordKind::PREFAB.0.record,
            "Prefab",
            RecordDescriptor {
                component: RecordReference::STUB,
                keys: Vec::new(),
            },
        ),
    ] {
        data.records.insert(Record {
            id,
            kind: RecordKind::RECORD,
            name: name.to_owned(),
            description: String::new(),
            data: descriptor.to_bytes(),
        });
    }
}
