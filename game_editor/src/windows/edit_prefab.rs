use std::sync::Arc;

use game_common::world::World;
use game_prefab::Prefab;
use game_ui::widgets::Callback;
use parking_lot::Mutex;

use super::record::EditState;
use super::world::OnWorldChangeEvent;

pub fn on_world_change_callback(edit_state: Arc<Mutex<EditState>>) -> Callback<OnWorldChangeEvent> {
    Callback::from(move |event: OnWorldChangeEvent| {
        let mut prefab = Prefab::new();

        for entity in event.entities {
            prefab.add(entity, &event.world);
        }

        let bytes = prefab.to_bytes();
        edit_state.lock().record.data = bytes;
    })
}

pub fn load_prefab(edit_state: &Arc<Mutex<EditState>>) -> World {
    let edit_state = edit_state.lock();

    let prefab = match Prefab::from_bytes(&edit_state.record.data) {
        Ok(prefab) => prefab,
        Err(err) => {
            tracing::warn!("invalid prefab data: {:?}", err);
            return World::default();
        }
    };

    let mut world = World::new();
    prefab.instantiate(&mut world);
    world
}
