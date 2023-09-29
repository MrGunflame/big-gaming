#![cfg(loom)]

use game_tasks::park::Parker;
use loom::thread;

#[test]
fn smoke() {
    loom::model(|| {
        let parker = Parker::new();
        let unparker = parker.unparker().clone();

        thread::spawn(move || {
            parker.park();
        });

        unparker.unpark();
    });
}
