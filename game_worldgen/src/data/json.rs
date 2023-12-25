use std::collections::HashMap;

use base64::alphabet::STANDARD;
use base64::engine::GeneralPurposeConfig;
use base64::Engine;
use game_common::components::components::{Components, RawComponent};
use game_common::components::Transform;
use game_common::record::RecordReference;
use game_common::world::entity::EntityKind;
use game_common::world::terrain::{Heightmap, TerrainMesh};
use game_common::world::CellId;
use glam::{IVec3, Quat, Vec3};
use serde::{Deserialize, Serialize};

use super::{Cells, Entity};

pub fn from_slice(slice: &[u8]) -> Result<super::Cells, Box<dyn std::error::Error>> {
    let root: JsonRoot = serde_json::from_slice(slice)?;

    let mut cells = HashMap::new();
    for cell in root.0 {
        let id = CellId::from_i32(IVec3::new(cell.id.x, cell.id.y, cell.id.z));

        let mut entities = Vec::new();
        for entity in cell.entities {
            let kind = match entity.kind {
                JsonEntityKind::Terrain => EntityKind::Terrain,
                JsonEntityKind::Actor => EntityKind::Actor,
                JsonEntityKind::Item => EntityKind::Item,
                JsonEntityKind::Object => EntityKind::Object,
            };

            if let Some(terrain) = entity.terrain {
                let mesh = load_heightmap(id, &terrain.mesh);

                entities.push(Entity {
                    id: RecordReference::STUB,
                    kind,
                    transform: Transform::default(),
                    components: Components::default(),
                    terrain: Some(mesh),
                });
                continue;
            }

            let id = entity.id.0.parse().unwrap();

            let transform = Transform {
                translation: Vec3::from_array(entity.transform.translation.0),
                rotation: Quat::from_array(entity.transform.rotation.0),
                scale: Vec3::from_array(entity.transform.scale.0),
            };

            let mut components = Components::new();
            for (id, component) in entity.components.0 {
                let id = id.0.parse().unwrap();

                components.insert(id, RawComponent::new(component));
            }

            entities.push(Entity {
                id,
                kind,
                transform,
                components,
                terrain: None,
            });
        }

        cells.insert(id, entities);
    }

    Ok(Cells { cells })
}

fn load_heightmap(id: CellId, input: &str) -> TerrainMesh {
    let engine = base64::engine::GeneralPurpose::new(&STANDARD, GeneralPurposeConfig::default());

    let buf = engine.decode(input).unwrap();

    let img = image::load_from_memory(&buf).unwrap().to_luma8();

    TerrainMesh {
        cell: id,
        offsets: Heightmap::from_image(img),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonRoot(pub Vec<JsonCell>);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonCell {
    pub id: JsonCellId,
    pub entities: Vec<JsonEntity>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonCellId {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonTransform {
    #[serde(default = "default_translation")]
    pub translation: JsonVec3,
    #[serde(default = "default_rotation")]
    pub rotation: JsonQuat,
    #[serde(default = "default_scale")]
    pub scale: JsonVec3,
}

impl Default for JsonTransform {
    fn default() -> Self {
        Self {
            translation: default_translation(),
            rotation: default_rotation(),
            scale: default_scale(),
        }
    }
}

fn default_translation() -> JsonVec3 {
    JsonVec3([0.0, 0.0, 0.0])
}

fn default_rotation() -> JsonQuat {
    JsonQuat([0.0, 0.0, 0.0, 1.0])
}

fn default_scale() -> JsonVec3 {
    JsonVec3([1.0, 1.0, 1.0])
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct JsonVec3(pub [f32; 3]);

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct JsonQuat(pub [f32; 4]);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonEntity {
    pub id: JsonRecordReference,
    #[serde(default)]
    pub transform: JsonTransform,
    #[serde(default)]
    pub components: JsonComponents,
    pub kind: JsonEntityKind,
    #[serde(default)]
    pub terrain: Option<JsonTerrain>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JsonRecordReference(pub String);

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JsonComponents(pub HashMap<JsonRecordReference, Vec<u8>>);

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JsonEntityKind {
    Terrain,
    Object,
    Actor,
    Item,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JsonTerrain {
    pub mesh: String,
}
