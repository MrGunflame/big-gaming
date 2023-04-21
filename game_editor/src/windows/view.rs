//! An immutable view of a scene.

use bevy::prelude::{
    AssetServer, Commands, Component, PointLight, PointLightBundle, Query, Res, Transform,
};
use bevy::scene::{Scene, SceneBundle};
use bevy_egui::EguiContext;
use game_data::uri::Uri;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ViewsWindowPlugin;

impl bevy::prelude::Plugin for ViewsWindowPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(render_view_windows);
    }
}

#[derive(Clone, Debug, Component)]
pub struct ViewWindow {
    setup: bool,
    handle: Uri,
}

impl ViewWindow {
    pub fn new(handle: Uri) -> Self {
        Self {
            setup: true,
            handle,
        }
    }
}

fn render_view_windows(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut windows: Query<(&mut EguiContext, &mut ViewWindow)>,
) {
    for (ctx, mut state) in &mut windows {
        if state.setup {
            state.setup = !state.setup;

            let handle = state.handle.as_ref().to_str().unwrap();
            let scene = assets.load::<Scene, _>(handle);

            commands.spawn(SceneBundle {
                scene,
                ..Default::default()
            });

            // Light
            commands.spawn(PointLightBundle {
                point_light: PointLight {
                    intensity: 1500.0,
                    shadows_enabled: true,
                    ..Default::default()
                },
                transform: Transform::from_xyz(4.0, 8.0, 4.0),
                ..Default::default()
            });
        }
    }
}
