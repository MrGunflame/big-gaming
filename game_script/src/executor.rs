use game_common::events::{ActionEvent, CollisionEvent, Event};
use game_common::record::RecordReference;
use game_common::world::entity::EntityBody;
use game_tracing::trace_span;

use crate::dependency::Dependencies;
use crate::effect::Effects;
use crate::scripts::RecordTargets;
use crate::{Context, Handle, ScriptServer};

#[derive(Debug)]
pub struct ScriptExecutor {
    server: ScriptServer,
    targets: RecordTargets,
}

impl ScriptExecutor {
    pub fn new(server: ScriptServer, targets: RecordTargets) -> Self {
        Self { server, targets }
    }

    pub fn run(&self, mut ctx: Context<'_>) -> Effects {
        let _span = trace_span!("ScriptExecutor::run").entered();

        let mut events = Vec::new();

        while let Some(event) = ctx.events.pop() {
            let handles = match &event {
                Event::Action(event) => self.call_action_event(&ctx, event),
                _ => continue,
            };

            for handle in handles {
                events.push(ExecScript {
                    handle,
                    event: event.clone(),
                });
            }
        }

        let mut effects = Effects::default();

        // TODO: Implement dependency tracking.
        let mut dependencies = Dependencies::default();

        for event in events {
            let mut instance = self
                .server
                .get(
                    &event.handle,
                    ctx.view,
                    ctx.physics_pipeline,
                    &mut effects,
                    &mut dependencies,
                    ctx.records,
                )
                .unwrap();

            instance.run(&event.event).unwrap();
        }

        effects
    }

    fn call_action_event(&self, ctx: &Context<'_>, event: &ActionEvent) -> &[Handle] {
        // The calling entity should exist.
        debug_assert!(ctx.view.get(event.invoker).is_some());
        self.targets.scripts.get(&event.action.0).unwrap()
    }
}

#[derive(Clone, Debug)]
struct ExecScript<'a> {
    handle: &'a Handle,
    event: Event,
}
