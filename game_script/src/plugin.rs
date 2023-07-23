use bevy_app::Plugin;
use game_common::events::{
    ActionEvent, CellLoadEvent, CellUnloadEvent, CollisionEvent, EquipEvent, Event, EventQueue,
    UnequipEvent,
};
use game_common::record::RecordReference;
use game_common::world::entity::EntityBody;

use crate::{Context, Handle};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ScriptPlugin;

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.insert_resource(EventQueue::new());
    }
}

pub fn flush_event_queue(mut ctx: Context<'_, '_>) {
    tracing::debug!("executing {} events", ctx.events.len());

    while let Some(event) = ctx.events.pop() {
        match event {
            Event::Action(event) => run_action(&mut ctx, event),
            Event::Collision(event) => run_collision(&mut ctx, event),
            Event::Equip(event) => run_equip(&mut ctx, event),
            Event::Unequip(event) => run_unequip(&mut ctx, event),
            Event::CellLoad(event) => run_cell_load(&mut ctx, event),
            Event::CellUnload(event) => run_cell_unload(&mut ctx, event),
        }
    }
}

fn run_action(ctx: &mut Context<'_, '_>, event: ActionEvent) {
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

    if let Some(actions) = ctx.record_targets.actions.get(&actor.race.0) {
        active_actions.extend(actions);
    }

    for (id, _) in entity.components.iter() {
        if let Some(actions) = ctx.record_targets.actions.get(&id) {
            active_actions.extend(actions);
        }
    }

    // TODO: We're only handling race/actor components here,
    // but we also must handle item actions.

    for action in active_actions {
        if action == event.action.0 {
            if let Some(scripts) = ctx.record_targets.scripts.get(&action) {
                for handle in scripts {
                    run_script(ctx, handle, &event.into());
                }
            }
        }
    }
}

fn run_collision(ctx: &mut Context<'_, '_>, event: CollisionEvent) {
    let Some(entity) = ctx.view.get(event.entity) else {
        tracing::warn!(
            "entity {:?} referenced by `CollisionEvent` {:?} does not exist",
            event.entity,
            event
        );

        return;
    };

    let mut active_scripts = vec![];

    if let EntityBody::Actor(actor) = &entity.body {
        if let Some(scripts) = ctx.record_targets.scripts.get(&actor.race.0) {
            active_scripts.extend(scripts);
        }
    }

    for (id, _) in entity.components.iter() {
        if let Some(scripts) = ctx.record_targets.scripts.get(&id) {
            active_scripts.extend(scripts);
        }
    }

    for handle in active_scripts {
        run_script(ctx, handle, &event.into());
    }
}

fn run_script(ctx: &mut Context<'_, '_>, handle: &Handle, event: &Event) {
    if let Some(mut instance) = ctx.server.get(handle, ctx.view, ctx.physics_pipeline) {
        if let Err(err) = instance.run(&event) {
            tracing::error!("failed to run script: {}", err);
        }
    }
}

fn run_equip(ctx: &mut Context<'_, '_>, event: EquipEvent) {
    todo!()
}

fn run_unequip(ctx: &mut Context<'_, '_>, event: UnequipEvent) {
    todo!()
}

fn run_cell_load(ctx: &mut Context<'_, '_>, event: CellLoadEvent) {}

fn run_cell_unload(ctx: &mut Context<'_, '_>, event: CellUnloadEvent) {}
