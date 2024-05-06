use game_wasm::components::builtin::{Collider, MeshInstance, RigidBody, RigidBodyKind, Transform};
use game_wasm::hierarchy::Children;
use game_wasm::math::Vec3;
use game_wasm::world::Entity;

pub struct Actor {}

pub struct SpawnActor {
    pub mesh: MeshInstance,
    pub collider: Collider,
    pub mesh_offset: Transform,
}

pub fn spawn_actor(actor: SpawnActor) -> Entity {
    let mesh = Entity::spawn();
    mesh.insert(actor.mesh);
    mesh.insert(actor.mesh_offset);

    let entity = Entity::spawn();
    entity.insert(actor.collider);
    entity.insert(RigidBody {
        kind: RigidBodyKind::Fixed,
        linvel: Vec3::ZERO,
        angvel: Vec3::ZERO,
    });

    let mut children = Children::new();
    children.insert(mesh.id());
    entity.insert(children);

    entity
}
