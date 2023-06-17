use std::borrow::Cow;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::Entity;
use bevy_ecs::query::With;
use bevy_ecs::system::{Commands, Query, ResMut};
use game_common::components::combat::Reload;
use game_common::components::player::HostPlayer;
use game_input::hotkeys::{
    Hotkey, HotkeyCode, HotkeyFilter, HotkeyId, HotkeyReader, Hotkeys, Key, TriggerKind,
};
use game_input::keyboard::KeyCode;
use game_input::mouse::MouseButton;

#[derive(Copy, Clone, Debug)]
pub struct CombatPlugin;

static mut ATTACK: Hotkey = Hotkey {
    id: HotkeyId(0),
    name: Cow::Borrowed("attack"),
    default: Key {
        trigger: TriggerKind::PRESSED,
        code: HotkeyCode::MouseButton {
            button: MouseButton::Left,
        },
    },
};

static mut RELOAD: Hotkey = Hotkey {
    id: HotkeyId(0),
    name: Cow::Borrowed("reload"),
    default: Key {
        trigger: TriggerKind::JUST_PRESSED,
        code: HotkeyCode::KeyCode {
            key_code: KeyCode::R,
        },
    },
};

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(register_events)
            // .add_system(attack_events)
            .add_system(reload_events);
    }
}

fn register_events(mut hotkeys: ResMut<Hotkeys>) {
    let mut attack = unsafe { &mut ATTACK };
    let id = hotkeys.register(attack.clone());
    attack.id = id;
    drop(attack);

    let mut reload = unsafe { &mut RELOAD };
    let id = hotkeys.register(reload.clone());
    reload.id = id;
    drop(reload);
}

struct AttackEvent;

impl HotkeyFilter for AttackEvent {
    fn filter(id: HotkeyId) -> bool {
        id == unsafe { ATTACK.id }
    }
}

struct ReloadEvent;

impl HotkeyFilter for ReloadEvent {
    fn filter(id: HotkeyId) -> bool {
        id == unsafe { RELOAD.id }
    }
}

// fn attack_events(
//     mut commands: Commands,
//     rapier: Res<RapierContext>,
//     players: Query<(Entity, &Transform, &ActorFigure), With<HostPlayer>>,
//     cameras: Query<&Transform, With<Camera3d>>,
//     mut events: HotkeyReader<AttackEvent>,
// ) {
//     if events.iter().count() == 0 {
//         return;
//     }

//     let (entity, player, figure) = players.single();
//     let cam = cameras.single();

//     let ray_origin = player.translation + figure.eyes;
//     let (y, x, _) = cam.rotation.to_euler(EulerRot::YXZ);
//     let ray_dir = Vec3::new(-y.sin() * x.cos(), x.sin(), -y.cos() * x.cos());
//     let max_toi = 1000.0;

//     let toi = match rapier.cast_ray(ray_origin, ray_dir, max_toi, true, QueryFilter::new()) {
//         Some((_, toi)) => toi,
//         None => max_toi,
//     };

//     let target = ray_origin + toi * ray_dir;

//     commands.entity(entity).insert(Attack { target });
// }

fn reload_events(
    mut commands: Commands,
    players: Query<Entity, With<HostPlayer>>,
    mut events: HotkeyReader<ReloadEvent>,
) {
    let entity = players.single();

    for _ in events.iter() {
        commands.entity(entity).insert(Reload);
    }
}
