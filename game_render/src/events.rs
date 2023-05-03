use glam::Vec2;

pub struct Events {
    pub cursor_enter: Option<Box<dyn Fn()>>,
    pub cursor_exit: Option<Box<dyn Fn()>>,
    pub click: Option<Box<dyn Fn()>>,
}

pub enum Event {}

pub fn hit_test(elem: Rect, cursor: Vec2) -> bool {
    // (elem.x + elem_size.x >= cursor.x)
    //     && (elem.x <= cursor.x + 1.0)
    //     && (elem.y + elem.y >= cursor.y)
    //     && (elem.y <= cursor.y + 1.0)

    cursor.x >= elem.min.x
        && cursor.x <= elem.max.x
        && cursor.y >= elem.min.y
        && cursor.y <= elem.max.y
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use crate::events::hit_test;

    use super::Rect;

    #[test]
    fn hit_test_edge() {
        let elem = Rect {
            min: Vec2 { x: 0.0, y: 0.0 },
            max: Vec2 { x: 1.0, y: 1.0 },
        };
        let cursor = Vec2 { x: 0.0, y: 0.0 };

        assert!(hit_test(elem, cursor));
    }

    #[test]
    fn hit_test_inside() {
        let elem = Rect {
            min: Vec2 { x: 0.0, y: 0.0 },
            max: Vec2 { x: 1.0, y: 1.0 },
        };
        let cursor = Vec2 { x: 0.5, y: 0.8 };

        assert!(hit_test(elem, cursor));
    }

    #[test]
    fn hit_test_outside() {
        let elem = Rect {
            min: Vec2 { x: 0.0, y: 0.0 },
            max: Vec2 { x: 1.0, y: 1.0 },
        };
        let cursor = Vec2 { x: 1.1, y: 0.5 };

        assert!(!hit_test(elem, cursor));
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}
