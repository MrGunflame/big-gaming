mod triangle;

use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::Renderer;

fn main() {
    let mut renderer = Renderer::new();
    renderer.entities.cameras.insert(Camera {
        transform: Default::default(),
        projection: Projection::default(),
        target: todo!(),
    });

    triangle::triangle();
}
