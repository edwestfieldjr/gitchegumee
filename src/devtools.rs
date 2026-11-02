//! Developer tools, built only with `--features dev`.
//!
//! - F12 saves `screenshot.png`.
//! - `DOWNBOUND_SHOT=<path>` saves a screenshot after `DOWNBOUND_SHOT_AFTER`
//!   seconds (default 3), then exits.
//! - `DOWNBOUND_SHOT_DIR=<dir>` saves one soon after each state change.
//! - `DOWNBOUND_AUTOPLAY=careful|careless` plays a whole run and exits after
//!   the memorial. `DOWNBOUND_SPEED=<n>` runs game time faster.
//! - `DOWNBOUND_JUMP=lost|reached`, with autoplay, goes straight to an ending.

use bevy::input::InputSystems;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use crate::content::{At, Content};
use crate::events::ActiveEvent;
use crate::states::GameState;
use crate::voyage::Ending;
use crate::voyage::route::LatLon;

const SHOT_DELAY: f32 = 1.2;
/// Real seconds to stay on the outcome screen, so its text can be captured.
const SECONDS_AT_OUTCOME: f32 = 26.0;
/// Seconds to wait for shaders to compile before autoplay starts.
const AUTOPLAY_WARMUP: f32 = 14.0;
const PRESS_EVERY: f32 = 0.4;
const SECONDS_AT_MEMORIAL: f32 = 8.0;

fn timed_shot(
    mut commands: Commands,
    time: Res<Time<Real>>,
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

/// Screenshot each screen shortly after it opens, and log each change.
fn state_shots(
    mut commands: Commands,
    time: Res<Time<Real>>,
    state: Res<State<GameState>>,
    mut shown: Local<Option<GameState>>,
    mut since: Local<f32>,
    mut pending: Local<bool>,
    mut count: Local<u32>,
    mut outcome_shots: Local<u32>,
) {
    if *shown != Some(*state.get()) {
        *shown = Some(*state.get());
        *since = time.elapsed_secs();
        *pending = true;
        info!("state: {:?}", state.get());
    }
    // The outcome screen is a timed sequence, so capture it repeatedly.
    let elapsed = time.elapsed_secs() - *since;
    if *state.get() == GameState::Outcome && elapsed >= *outcome_shots as f32 + 1.0 {
        *outcome_shots += 1;
        *pending = true;
    }
    if *pending && time.elapsed_secs() >= *since + SHOT_DELAY {
        *pending = false;
        if let Ok(dir) = std::env::var("DOWNBOUND_SHOT_DIR") {
            *count += 1;
            let path = format!("{dir}/{:03}_{:?}.png", *count, state.get());
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
    }
}

#[derive(Default)]
struct Autoplay {
    last_press: f32,
    held: Vec<KeyCode>,
    port_step: usize,
    memorial_since: Option<f32>,
    outcome_since: Option<f32>,
    ran_all_pumps: bool,
}

/// Press keys through the normal input resource, so the game's own input
/// handling is what gets exercised.
fn autoplay(
    time: Res<Time<Real>>,
    state: Res<State<GameState>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
    mut next: ResMut<NextState<GameState>>,
    active: Option<Res<ActiveEvent>>,
    content: Option<Res<Content>>,
    mut play: Local<Autoplay>,
) {
    let Ok(mode) = std::env::var("DOWNBOUND_AUTOPLAY") else {
        return;
    };
    let careful = mode == "careful";
    let now = time.elapsed_secs();
    for key in std::mem::take(&mut play.held) {
        keys.release(key);
    }
    if now < AUTOPLAY_WARMUP {
        return;
    }
    if *state.get() == GameState::Voyage {
        keys.press(KeyCode::Space);
    } else {
        keys.release(KeyCode::Space);
    }
    if now - play.last_press < PRESS_EVERY {
        return;
    }
    let jump = std::env::var("DOWNBOUND_JUMP").ok();
    if *state.get() == GameState::Title
        && let Some(jump) = jump.as_deref()
    {
        commands.insert_resource(Ending {
            reached_bay: jump == "reached",
            minute: At(10, 19, 20).minute(),
            position: LatLon {
                lat: 47.0,
                lon: -85.1,
            },
        });
        next.set(GameState::Outcome);
        return;
    }
    let key = match state.get() {
        GameState::Boot => return,
        // The careful answer to the damage report is every pump, the second choice.
        GameState::Event
            if careful
                && !play.ran_all_pumps
                && active
                    .as_ref()
                    .zip(content.as_ref())
                    .is_some_and(|(active, content)| content.events[active.0].id == "topside") =>
        {
            play.ran_all_pumps = true;
            KeyCode::ArrowDown
        }
        GameState::Port if careful => {
            // All clamps, north-shore route, then sail.
            const STEPS: [KeyCode; 6] = [
                KeyCode::ArrowRight,
                KeyCode::ArrowRight,
                KeyCode::ArrowDown,
                KeyCode::ArrowRight,
                KeyCode::ArrowDown,
                KeyCode::Enter,
            ];
            let key = STEPS[play.port_step.min(STEPS.len() - 1)];
            play.port_step += 1;
            key
        }
        GameState::Voyage => return,
        GameState::Helm => {
            if careful {
                KeyCode::ArrowDown
            } else {
                return;
            }
        }
        GameState::Outcome => {
            let since = *play.outcome_since.get_or_insert(now);
            if now - since < SECONDS_AT_OUTCOME {
                return;
            }
            KeyCode::Enter
        }
        GameState::Memorial => {
            let since = *play.memorial_since.get_or_insert(now);
            if now - since > SECONDS_AT_MEMORIAL {
                exit.write(AppExit::Success);
            }
            return;
        }
        _ => KeyCode::Enter,
    };
    keys.press(key);
    play.held.push(key);
    play.last_press = now;
}

fn set_speed(mut time: ResMut<Time<Virtual>>, mut done: Local<bool>) {
    if std::mem::replace(&mut *done, true) {
        return;
    }
    if let Some(speed) = std::env::var("DOWNBOUND_SPEED")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
    {
        let pace = time.relative_speed();
        time.set_relative_speed(pace * speed);
        time.set_max_delta(std::time::Duration::from_millis(500));
    }
}

pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Title), set_speed)
            .add_systems(PreUpdate, autoplay.after(InputSystems))
            .add_systems(Update, (timed_shot, key_shot, state_shots));
    }
}
