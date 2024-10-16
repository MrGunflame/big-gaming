use std::process::Command;

use game_core::logger::ipc::Sender;

pub fn pre_exec(cmd: &mut Command, sender: Sender) {
    let sender = sender.into_fd();

    cmd.env("FD", val);
}
