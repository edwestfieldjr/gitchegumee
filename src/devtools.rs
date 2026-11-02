//! Developer tools, built only with `--features dev`.
//!
//! `DOWNBOUND_SHOT=<path>` saves a screenshot after `DOWNBOUND_SHOT_AFTER`
//! seconds (default 3), then exits. F12 saves one to `screenshot.png`.

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

fn timed_shot(
    mut commands: Commands,
    time: Res<Time>,
    mut taken_at: Local<Option<f32>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(path) = std::env::var("DOWNBOUND_SHOT") else {
        return;
    };
    let after = std::env::var("DOWNBOUND_SHOT_AFTER")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(3.0);
    match *taken_at {
        None if time.elapsed_secs() >= after => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
            *taken_at = Some(time.elapsed_secs());
        }
        Some(when) if time.elapsed_secs() >= when + 1.0 => {
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}

fn key_shot(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::F12) {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk("screenshot.png"));
    }
}

pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (timed_shot, key_shot));
    }
}
