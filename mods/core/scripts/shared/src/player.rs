use game_wasm::action::Action;
use game_wasm::components::builtin::{
    Axis, Capsule, Collider, ColliderShape, MeshInstance, Transform,
};
use game_wasm::components::{Components, RawComponent};
use game_wasm::encoding::{Decode, Encode};
use game_wasm::entity::EntityId;
use game_wasm::events::{Event, PlayerConnect};
use game_wasm::inventory::{Inventory, ItemStack};
use game_wasm::math::{Quat, Vec3};
use game_wasm::player::PlayerId;
use game_wasm::world::{Entity, RecordReference};

use crate::actor::{spawn_actor, SpawnActor};
use crate::components::{
    EVENT_GUN_EQUIP, EVENT_GUN_UNEQUIP, PLAYER_RESPAWN, TEST_WEAPON, TRANSFORM_CHANGED,
};
use crate::{
    assets, Camera, CharacterController, Equippable, GunProperties, Health, Humanoid,
    LookingDirection, MovementSpeed, PlayerCamera, Projectile, SpawnPoint,
};

pub fn spawn_player(_: EntityId, event: PlayerConnect) {
    let transform = Transform {
        translation: Vec3::new(1.0, 10.0, 1.0),
        ..Default::default()
    };
    let health = Health {
        value: 10,
        max: 100,
    };

    let mut inventory = Inventory::new();

    {
        let id = inventory.insert(ItemStack {
            item: TEST_WEAPON,
            equipped: false,
            hidden: false,
            quantity: 1,
            components: Components::default(),
        });

        let slot = inventory.get_mut(id).unwrap();

        let mut buf = RawComponent::default();
        buf.write(GunProperties {
            damage: 1.0,
            cooldown: 1.0,
            magazine_capacity: 30,
            projectile: Projectile {
                id: RecordReference::STUB,
                translation: Vec3::ZERO.to_array(),
                rotation: Quat::IDENTITY.to_array(),
            },
        });

        slot.components.insert(GunProperties {
            damage: 1.0,
            cooldown: 1.0,
            magazine_capacity: 30,
            projectile: Projectile {
                id: RecordReference::STUB,
                translation: Vec3::ZERO.to_array(),
                rotation: Quat::IDENTITY.to_array(),
            },
        });
        slot.components.insert(Equippable {
            on_equip: EVENT_GUN_EQUIP,
            on_uneqip: EVENT_GUN_UNEQUIP,
        });
    }

    let player = SpawnPlayer {
        old_entity: None,
        transform,
        inventory,
        health,
    };

    player.spawn(Some(event.player));
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct TransformChanged {
    pub entity: EntityId,
}

impl Event for TransformChanged {
    const ID: RecordReference = TRANSFORM_CHANGED;
}

pub fn update_camera_transform(_: EntityId, event: TransformChanged) {
    let entity = Entity::new(event.entity);
    let Ok(mut transform) = entity.get::<Transform>() else {
        return;
    };
    let Ok(camera) = entity.get::<PlayerCamera>() else {
        return;
    };

    transform.translation += camera.offset;
    transform.rotation = camera.rotation;
    let camera = Entity::new(camera.camera);
    camera.insert(transform);

    entity.insert(LookingDirection {
        translation: transform.translation,
        rotation: transform.rotation,
    });
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct RespawnPlayer;

impl Action for RespawnPlayer {
    const ID: RecordReference = PLAYER_RESPAWN;
}

pub fn respawn_player(entity: EntityId, RespawnPlayer: RespawnPlayer) {
    let Ok(camera) = Entity::new(entity).get::<Camera>() else {
        return;
    };

    let actor = Entity::new(camera.parent);

    let Ok(respawn_point) = actor.get::<SpawnPoint>() else {
        return;
    };

    SpawnPlayer {
        old_entity: Some(actor.id()),
        transform: Transform::from_translation(respawn_point.translation),
        inventory: Inventory::default(),
        health: Health {
            value: 10,
            max: 100,
        },
    }
    .spawn(None);
}

#[derive(Clone, Debug)]
pub struct SpawnPlayer {
    old_entity: Option<EntityId>,
    transform: Transform,
    inventory: Inventory,
    health: Health,
}

impl SpawnPlayer {
    pub fn spawn(self, player: Option<PlayerId>) {
        let entity = spawn_actor(SpawnActor {
            mesh: MeshInstance {
                model: assets::RESOURCE_PERSON.into(),
            },
            collider: Collider {
                friction: 1.0,
                restitution: 1.0,
                shape: ColliderShape::Capsule(Capsule {
                    axis: Axis::Y,
                    half_height: 0.5,
                    radius: 0.5,
                }),
            },
            mesh_offset: Transform::from_translation(Vec3::new(0.0, -1.0, 0.0)),
        });

        self.init_camera(&entity, player);

        entity.insert(self.transform);
        entity.insert(MovementSpeed(1.0));
        entity.insert(Humanoid);
        entity.insert(CharacterController);
        entity.insert(self.health);
        entity.insert(SpawnPoint {
            translation: Vec3::new(1.0, 10.0, 1.0),
        });
        entity.insert(LookingDirection::default());
        entity.insert(self.inventory);
    }

    fn init_camera(&self, actor: &Entity, player: Option<PlayerId>) {
        let camera = if let Some(entity) = self.old_entity {
            let entity = Entity::new(entity);
            let player_camera = entity.get::<PlayerCamera>().unwrap();
            entity.remove::<PlayerCamera>();

            Entity::new(player_camera.camera)
        } else {
            let camera = Entity::spawn();
            camera.insert(Transform::default());
            camera.insert(Humanoid);

            player.unwrap().set_active(camera.id());

            camera
        };

        actor.insert(PlayerCamera {
            camera: camera.id(),
            offset: Vec3::new(0.0, 0.8, 0.0),
            rotation: Quat::IDENTITY,
        });

        camera.insert(Camera { parent: actor.id() });
    }
}
