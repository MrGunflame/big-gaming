use game_net::snapshot::CommandQueue;

use super::connect;
use crate::spawn_server;

// #[tokio::test]
// async fn respect_overrides() {
//     pretty_env_logger::init();

//     spawn_server();

//     let queue = CommandQueue::new();
//     let handle = connect(queue.clone());

//     //     loop {
//     //         if let Some(it) = queue.pop() {
//     //             dbg!(it);
//     //         }
//     //     }
//     tokio::signal::ctrl_c().await.unwrap();
// }
