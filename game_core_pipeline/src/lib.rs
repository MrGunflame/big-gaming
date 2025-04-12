pub mod camera;
pub mod entities;
pub mod lights;
pub mod material;

mod passes;

use entities::Entities;
use game_render::Renderer;
use material::StandardMaterial;

pub fn init(renderer: &mut Renderer) -> Entities {
    let (entities, rx) = Entities::new();

    renderer.with_command_queue_and_graph(move |graph, queue| {
        passes::init(graph, queue, rx);
    });

    entities
}
