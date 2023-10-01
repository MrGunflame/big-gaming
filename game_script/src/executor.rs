use game_common::events::{ActionEvent, CollisionEvent, Event};
use game_common::record::RecordReference;
use game_common::world::entity::EntityBody;
use game_tracing::trace_span;

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

    pub fn run(&self, mut ctx: Context<'_, '_>) -> Effects {
        let _span = trace_span!("ScriptExecutor::run").entered();

        let mut events = Vec::new();

        while let Some(event) = ctx.events.pop() {
            match event {
                Event::Action(event) => self.queue_action(event, &mut ctx, &mut events),
                _ => (),
            }
        }

        let mut effects = Effects::default();

        for event in events {
            let mut instance = self
                .server
                .get(&event.handle, ctx.view, ctx.physics_pipeline, &mut effects)
                .unwrap();

            instance.run(&event.event).unwrap();
        }

        effects
    }

    fn queue_action(
        &self,
        event: ActionEvent,
        ctx: &mut Context<'_, '_>,
        events: &mut Vec<FireEvent>,
    ) {
        let Some(entity) = ctx.view.get(event.invoker) else {
            tracing::warn!(
                "entity {:?} referenced by `ActionEvent` {:?} does not exist",
                event.invoker,
                event
            );

            return;
        };

        let actor = match &entity.body {
            EntityBody::Actor(actor) => actor,
            _ => {
                tracing::warn!(
                    "`ActionEvent` must be an actor, but was {:?}",
                    entity.body.kind()
                );

                return;
            }
        };

        let mut active_actions: Vec<RecordReference> = vec![];

        if let Some(actions) = self.targets.actions.get(&actor.race.0) {
            active_actions.extend(actions);
        }

        for (id, _) in entity.components.iter() {
            if let Some(actions) = self.targets.actions.get(&id) {
                active_actions.extend(actions);
            }
        }

        // TODO: We're only handling race/actor components here,
        // but we also must handle item actions.

        for action in active_actions {
            if action == event.action.0 {
                if let Some(scripts) = self.targets.scripts.get(&action) {
                    for handle in scripts {
                        events.push(FireEvent {
                            handle: handle.clone(),
                            event: event.into(),
                        });
                    }
                }
            }
        }
    }

    fn queue_collision(
        &self,
        event: CollisionEvent,
        ctx: &mut Context<'_, '_>,
        events: &mut Vec<FireEvent>,
    ) {
        let Some(entity) = ctx.view.get(event.entity) else {
            tracing::warn!(
                "entity {:?} referenced by `CollisionEvent` {:?} does not eixst",
                event.entity,
                event
            );
            return;
        };
    }
}

#[derive(Clone, Debug)]
struct FireEvent {
    handle: Handle,
    event: Event,
}
