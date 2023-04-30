use game_render::window::*;
use tokio::runtime::{Builder, Runtime};

fn main() {
    let rt = Builder::new_current_thread().build().unwrap();

    rt.block_on(run());
}
