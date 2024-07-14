use std::sync::Arc;

use game_scene2::Prefab;
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
