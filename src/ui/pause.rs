//! Pausing, on any screen, and the key hints that are always on show.

use bevy::input::InputSystems;
use bevy::prelude::*;

use super::palette::Pal;
use super::{Label, VIEW_W, advance_pressed, at, rect};
use crate::states::{GameState, PauseState};
use crate::ui::font::CELL_W;

/// The pause panel draws over everything except the opening wipe, including
/// the text of the screen underneath.
const PANEL_Z: f32 = 70.0;
const PANEL_W: f32 = 124.0;
const PANEL_Y: f32 = 50.0;
const PANEL_H: f32 = 28.0;
const RESUME_HINT: &str = "P OR ENTER: RESUME";
/// The key hints sit in the bottom right corner, inside the event panel's frame.
const HINT_X: f32 = 270.0;
const HINT_Y: [f32; 2] = [178.0, 186.0];

/// The always-visible mute hint. The audio plugin updates it to show the state.
#[derive(Component)]
pub struct MuteHint;

/// Reads the pause keys. Other plugins that press keys for testing run before it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PauseKeys;

/// Screens read their keys in `Update`, so taking the presses away here keeps
/// every screen still while paused, and keeps the key that resumes the game from
/// also acting on the screen. Mute still works during a pause.
fn swallow_presses(keys: &mut ButtonInput<KeyCode>) {
    let pressed: Vec<KeyCode> = keys
        .get_just_pressed()
        .copied()
        .filter(|key| *key != KeyCode::KeyM)
        .collect();
    for key in pressed {
        keys.clear_just_pressed(key);
    }
}

fn pause_keys(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    game: Res<State<GameState>>,
    pause: Res<State<PauseState>>,
    mut next: ResMut<NextState<PauseState>>,
) {
    if *game.get() == GameState::Boot {
        return;
    }
    let toggle = keys.any_just_pressed([KeyCode::KeyP, KeyCode::Escape]);
    match pause.get() {
        PauseState::Running => {
            if toggle {
                next.set(PauseState::Paused);
            }
        }
        PauseState::Paused => {
            if toggle || advance_pressed(&keys) {
                next.set(PauseState::Running);
            }
            swallow_presses(&mut keys);
        }
    }
}

/// Stopping virtual time stops the simulation, which runs on `FixedUpdate`, along
/// with everything drawn from `Time`.
fn stop_game_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

fn start_game_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

fn panel_text(text: &str, y: f32, color: Pal) -> impl Bundle {
    let width = text.len() as f32 * CELL_W;
    let x = ((VIEW_W as f32 - width) / 2.0).floor();
    (
        Label::new(text, color),
        at(x, y, PANEL_Z + 2.0),
        DespawnOnExit(PauseState::Paused),
    )
}

fn show_panel(mut commands: Commands) {
    let scope = DespawnOnExit(PauseState::Paused);
    let x = ((VIEW_W as f32 - PANEL_W) / 2.0).floor();
    commands.spawn((
        rect(x, PANEL_Y, PANEL_W, PANEL_H, Pal::Foam, PANEL_Z),
        scope.clone(),
    ));
    commands.spawn((
        rect(
            x + 2.0,
            PANEL_Y + 2.0,
            PANEL_W - 4.0,
            PANEL_H - 4.0,
            Pal::Night,
            PANEL_Z + 1.0,
        ),
        scope,
    ));
    commands.spawn(panel_text("PAUSED", PANEL_Y + 5.0, Pal::White));
    commands.spawn(panel_text(RESUME_HINT, PANEL_Y + 15.0, Pal::Gray));
}

fn spawn_key_hints(mut commands: Commands) {
    let [pause_y, mute_y] = HINT_Y;
    commands.spawn((
        Label::new("P PAUSE", Pal::Gray),
        at(HINT_X, pause_y, PANEL_Z + 2.0),
    ));
    commands.spawn((
        Label::new("M MUTE", Pal::Gray),
        at(HINT_X, mute_y, PANEL_Z + 2.0),
        MuteHint,
    ));
}

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(GameState::Boot), spawn_key_hints)
            .add_systems(OnEnter(PauseState::Paused), (stop_game_time, show_panel))
            .add_systems(OnExit(PauseState::Paused), start_game_time)
            .add_systems(PreUpdate, pause_keys.in_set(PauseKeys).after(InputSystems));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_resource::<Time<Virtual>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_state::<GameState>()
            .init_state::<PauseState>()
            .add_systems(OnEnter(PauseState::Paused), stop_game_time)
            .add_systems(OnExit(PauseState::Paused), start_game_time)
            .add_systems(PreUpdate, pause_keys);
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Title);
        app.update();
        app
    }

    /// One frame with a key newly pressed. The key is then let go.
    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
    }

    fn is_paused(app: &App) -> bool {
        app.world().resource::<Time<Virtual>>().is_paused()
    }

    #[test]
    fn p_pauses_and_resumes_game_time() {
        let mut app = test_app();
        assert!(!is_paused(&app));
        press(&mut app, KeyCode::KeyP);
        assert!(is_paused(&app));
        press(&mut app, KeyCode::KeyP);
        assert!(!is_paused(&app));
    }

    #[test]
    fn escape_and_enter_also_work() {
        let mut app = test_app();
        press(&mut app, KeyCode::Escape);
        assert!(is_paused(&app));
        press(&mut app, KeyCode::Enter);
        assert!(!is_paused(&app));
    }

    #[test]
    fn enter_does_not_pause_a_running_game() {
        let mut app = test_app();
        press(&mut app, KeyCode::Enter);
        assert!(!is_paused(&app));
    }

    #[test]
    fn a_pause_hides_every_key_from_the_screen_except_mute() {
        let mut keys = ButtonInput::<KeyCode>::default();
        for key in [KeyCode::Enter, KeyCode::ArrowUp, KeyCode::KeyM] {
            keys.press(key);
        }
        swallow_presses(&mut keys);
        assert!(!keys.just_pressed(KeyCode::Enter));
        assert!(!keys.just_pressed(KeyCode::ArrowUp));
        assert!(keys.just_pressed(KeyCode::KeyM));
    }

    #[test]
    fn the_panel_text_fits_its_box() {
        assert!(RESUME_HINT.len() as f32 * CELL_W + 8.0 <= PANEL_W);
    }
}
