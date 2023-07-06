use glam::Vec2;

/// Remap from absolute screen space to OpenGL vertex translations.
pub fn remap(input: Vec2, size: Vec2) -> Vec2 {
    let x = lerp(-1.0, 1.0, input.x / size.x);
    let y = lerp(1.0, -1.0, input.y / size.y);
    Vec2::new(x, y)
}

fn lerp(lhs: f32, rhs: f32, s: f32) -> f32 {
    lhs + ((rhs - lhs) * s)
}

#[cfg(test)]
mod tests {
    use game_common::assert_approx_eq;
    use glam::Vec2;

    use super::remap;

    #[test]
    fn remap_baseline() {
        let input = Vec2::new(0.0, 0.0);
        let size = Vec2::new(1000.0, 1000.0);

        assert_approx_eq!(remap(input, size), Vec2::new(-1.0, 1.0));
    }

    #[test]
    fn remap_center() {
        let input = Vec2::new(500.0, 500.0);
        let size = Vec2::splat(1000.0);

        assert_approx_eq!(remap(input, size), Vec2::new(0.0, 0.0));
    }

    // What a great test name.
    #[test]
    fn remap_elsewhere() {
        let input = Vec2::new(400.0, 600.0);
        let size = Vec2::splat(1000.0);

        assert_approx_eq!(remap(input, size), Vec2::new(-0.2, -0.2));
    }
}
