use std::sync::Arc;

use super::{NodeId, Scope};

pub fn create_effect<F>(f: F)
where
    F: Fn() + Send + Sync + 'static,
{
    dbg!("r");

    let effect = Effect {
        f: Arc::new(f),
        signals: vec![],
        is_first_run: true,
    };

    let id = super::with_runtime(|rt| rt.effects.insert(effect));

    super::run_effect(NodeId(id));
}

#[derive(Clone)]
pub(super) struct Effect {
    pub(super) f: Arc<dyn Fn() + Send + Sync + 'static>,
    pub(super) signals: Vec<NodeId>,
    pub(super) is_first_run: bool,
}
