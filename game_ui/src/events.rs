use std::sync::Arc;

use game_input::keyboard::KeyboardInput;
use game_input::mouse::{MouseButtonInput, MouseWheel};
use game_render::camera::RenderTarget;
use game_window::cursor::Cursor;
use game_window::events::CursorMoved;
use game_window::windows::WindowId;
use glam::Vec2;

use crate::reactive::{Context, Event, Runtime};
use crate::render::Rect;

impl Event for KeyboardInput {}
impl Event for MouseButtonInput {}
impl Event for MouseWheel {}
impl Event for CursorMoved {}

pub(crate) fn call_events<E>(window: WindowId, runtime: &Runtime, cursor: &Arc<Cursor>, event: E)
where
    E: Event + Clone,
{
    let mut handlers = Vec::new();

    // Collect all event handlers first, then release the mutex lock
    // before calling them. Handlers may call runtime functions, which
    // would deadlock if the still held the lock.
    {
        let rt = &mut *runtime.inner.lock();

        let Some(window) = rt.windows.get(&RenderTarget::Window(window)) else {
            return;
        };

        for doc_id in &window.documents {
            let doc = rt.documents.get(doc_id.0).unwrap();

            if let Some(ids) = doc.event_handlers.get::<E>() {
                for id in ids {
                    let handler = rt.get_event_handler(*id);
                    handlers.push((*doc_id, None, handler));
                }
            }

            // for (key, layout) in doc.layout.keys().zip(doc.layout.layouts()) {
            //     let aabb = Rect {
            //         min: layout.position,
            //         max: UVec2 {
            //             x: layout.position.x + layout.width,
            //             y: layout.position.y + layout.height,
            //         },
            //     };

            //     if !hit_test(aabb, cursor.position()) {
            //         continue;
            //     }

            //     let node_id = *doc.layout_node_map2.get(&key).unwrap();
            //     let node = rt.nodes.get_mut(node_id.0).unwrap();

            //     if let Some(handler) = node.get() {
            //         handlers.push((*doc_id, Some(node_id), handler));
            //     }
            // }
        }
    }

    // Handle events bottom-up; The children get to handle their events
    // before the parents do.
    for (document, node, handler) in handlers.into_iter().rev() {
        handler.call(Context {
            event: event.clone(),
            node,
            document,
            runtime: runtime.clone(),
        });
    }
}

pub fn hit_test(elem: Rect, cursor: Vec2) -> bool {
    // FIXME: Should this maybe be int cmp?
    // Floats are only relevant if cursor really uses its
    // full float range.
    cursor.x >= elem.min.x as f32
        && cursor.x <= elem.max.x as f32
        && cursor.y >= elem.min.y as f32
        && cursor.y <= elem.max.y as f32
}

#[cfg(test)]
mod tests {
    use glam::UVec2;
    use glam::Vec2;

    use super::hit_test;

    use super::Rect;

    #[test]
    fn hit_test_edge() {
        let elem = Rect {
            min: UVec2 { x: 0, y: 0 },
            max: UVec2 { x: 1, y: 1 },
        };
        let cursor = Vec2 { x: 0.0, y: 0.0 };

        assert!(hit_test(elem, cursor));
    }

    #[test]
    fn hit_test_inside() {
        let elem = Rect {
            min: UVec2 { x: 0, y: 0 },
            max: UVec2 { x: 1, y: 1 },
        };
        let cursor = Vec2 { x: 0.5, y: 0.8 };

        assert!(hit_test(elem, cursor));
    }

    #[test]
    fn hit_test_outside() {
        let elem = Rect {
            min: UVec2 { x: 0, y: 0 },
            max: UVec2 { x: 1, y: 1 },
        };
        let cursor = Vec2 { x: 1.1, y: 0.5 };

        assert!(!hit_test(elem, cursor));
    }
}
