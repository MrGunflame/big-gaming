#![cfg(loom)]

use game_tasks::park::Parker;
use loom::sync::Arc;
use loom::thread;

#[test]
fn smoke() {
    loom::model(|| {
        let parker = Arc::new(Parker::new());
        let unparker = parker.clone();

        thread::spawn(move || {
            parker.park();
        });

        unparker.unpark();
    });
}
