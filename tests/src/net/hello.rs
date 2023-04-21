use std::time::Duration;

use game_net::snapshot::CommandQueue;

use crate::{spawn_server, timeout};

use super::connect;

pub async fn hello() {
    pretty_env_logger::init();

    spawn_server();

    let queue = CommandQueue::new();
    let _handle = connect(queue.clone());

    timeout(Duration::from_secs(15)).await;
}
