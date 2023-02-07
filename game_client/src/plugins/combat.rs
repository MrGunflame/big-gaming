use std::borrow::Cow;

use bevy::prelude::{
    Bundle, Camera3d, Commands, Entity, EulerRot, MouseButton, Plugin, Query, Res, ResMut,
    Transform, Vec3, With,
};
use bevy_rapier3d::prelude::{QueryFilter, RapierContext};
use game_common::components::actor::ActorFigure;
use game_common::components::combat::{Attack, Health, IncomingDamage, Resistances};
use game_common::components::player::HostPlayer;
use game_input::hotkeys::{
    Hotkey, HotkeyCode, HotkeyFilter, HotkeyId, HotkeyReader, Hotkeys, Key, TriggerKind,
};

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

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(register_events)
            .add_system(attack_events);
    }
}

fn register_events(mut hotkeys: ResMut<Hotkeys>) {
    let mut attack = unsafe { &mut ATTACK };
    let id = hotkeys.register(attack.clone());
    attack.id = id;
    drop(attack);
}

struct AttackEvent;

impl HotkeyFilter for AttackEvent {
    fn filter(id: HotkeyId) -> bool {
        id == unsafe { ATTACK.id }
    }
}

fn attack_events(
    mut commands: Commands,
    rapier: Res<RapierContext>,
    players: Query<(Entity, &Transform, &ActorFigure), With<HostPlayer>>,
    cameras: Query<&Transform, With<Camera3d>>,
    mut events: HotkeyReader<AttackEvent>,
) {
    if events.iter().count() == 0 {
        return;
    }

    let (entity, player, figure) = players.single();
    let cam = cameras.single();

    let ray_origin = player.translation + figure.eyes;
    let (y, x, _) = cam.rotation.to_euler(EulerRot::YXZ);
    let ray_dir = Vec3::new(-y.sin() * x.cos(), x.sin(), -y.cos() * x.cos());
    let max_toi = 1000.0;

    let toi = match rapier.cast_ray(ray_origin, ray_dir, max_toi, true, QueryFilter::new()) {
        Some((_, toi)) => toi,
        None => max_toi,
    };

    let target = ray_origin + toi * ray_dir;

    commands.entity(entity).insert(Attack { target });
}

#[derive(Bundle)]
pub struct CombatBundle {
    pub incoming_damage: IncomingDamage,
    pub health: Health,
    pub resistances: Resistances,
}

impl CombatBundle {
    pub fn new() -> Self {
        Self {
            incoming_damage: IncomingDamage::new(),
            health: Health::new(50),
            resistances: Resistances::new(),
        }
    }
}
