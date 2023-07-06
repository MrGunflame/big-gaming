use std::time::Duration;

use bevy_app::App;
use game_server::config::Config;
use tokio::runtime::Runtime;

mod net;

fn main() {
    let tests = [net::hello::hello];

    for test in tests {
        let rt = Runtime::new().unwrap();
        rt.block_on(test());
        println!("ok");
    }
}

fn spawn_server() {
    tokio::task::spawn(async move {
        let app = App::new();

        game_server::run(app, Config::default()).await;
    });
}

async fn timeout(duration: Duration) {
    tokio::time::sleep(duration.into()).await
}
