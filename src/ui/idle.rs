//! The game never waits on the player for long. Once a screen has sat for
//! [`IDLE_SECONDS`] with its text finished and no key pressed, this presses Enter,
//! which gives whatever is selected, so the default unless the player moved it.
//!
//! The press goes through the same input as a keyboard, so each screen keeps its
//! own rules for what Enter does. A screen that is still playing something the
//! player should see through sends [`Busy`] to hold the timer back.

use bevy::input::InputSystems;
use bevy::prelude::*;

use super::pause::PauseKeys;
use super::typing::Typed;
use crate::states::{EventPhase, GameState, MemorialPhase, PauseState};

/// Real seconds a screen waits before it moves on by itself.
pub const IDLE_SECONDS: f32 = 10.0;
/// Real seconds between presses once the wait is over. Some screens take two, as
/// the port menu does to reach SAIL, and the first must not start the wait again.
const REPRESS_SECONDS: f32 = 0.6;
/// The most one frame can add. A hidden browser tab stops drawing frames, and
/// coming back to it must not count as the player having waited all that time.
const LONGEST_FRAME: f32 = 0.25;

/// This screen is still playing, so the wait hasn't begun. Send it every frame.
#[derive(Message, Debug, Clone, Copy)]
pub struct Busy;

#[derive(Debug, Default)]
struct IdleClock {
    waited: f32,
}

impl IdleClock {
    /// Move on by `seconds`, and say whether it's time to press.
    fn step(&mut self, seconds: f32, waiting: bool) -> bool {
        if !waiting {
            self.waited = 0.0;
            return false;
        }
        self.waited += seconds.min(LONGEST_FRAME);
        if self.waited < IDLE_SECONDS {
            return false;
        }
        self.waited = IDLE_SECONDS - REPRESS_SECONDS;
        true
    }
}

/// The helm plays by itself and never asks for Enter.
fn screen_can_wait(game: GameState, pause: PauseState) -> bool {
    pause == PauseState::Running && !matches!(game, GameState::Boot | GameState::Helm)
}

type Screen = (GameState, Option<EventPhase>, Option<MemorialPhase>);

fn press_when_idle(
    real: Res<Time<Real>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    game: Res<State<GameState>>,
    pause: Res<State<PauseState>>,
    event: Option<Res<State<EventPhase>>>,
    memorial: Option<Res<State<MemorialPhase>>>,
    typed: Query<&Typed>,
    mut busy: MessageReader<Busy>,
    mut last_screen: Local<Option<Screen>>,
    mut clock: Local<IdleClock>,
    mut pressing: Local<bool>,
) {
    if std::mem::take(&mut *pressing) {
        keys.release(KeyCode::Enter);
    }
    let screen = (
        *game.get(),
        event.map(|phase| *phase.get()),
        memorial.map(|phase| *phase.get()),
    );
    let same_screen = last_screen.replace(screen) == Some(screen);
    let playing = busy.read().count() > 0;
    let waiting = same_screen
        && !playing
        && keys.get_just_pressed().next().is_none()
        && screen_can_wait(*game.get(), *pause.get())
        && typed.iter().all(Typed::finished);
    if clock.step(real.delta_secs(), waiting) {
        keys.press(KeyCode::Enter);
        *pressing = true;
    }
}

pub struct IdlePlugin;

impl Plugin for IdlePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Busy>().add_systems(
            PreUpdate,
            press_when_idle.after(InputSystems).before(PauseKeys),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How many presses come out of `seconds` of waiting, in steady frames.
    fn presses(clock: &mut IdleClock, seconds: f32) -> usize {
        let frame = 0.1;
        (0..(seconds / frame).round() as usize)
            .filter(|_| clock.step(frame, true))
            .count()
    }

    #[test]
    fn nothing_happens_before_the_wait_is_over() {
        assert_eq!(presses(&mut IdleClock::default(), IDLE_SECONDS - 0.5), 0);
    }

    #[test]
    fn the_wait_ends_in_a_press() {
        assert_eq!(presses(&mut IdleClock::default(), IDLE_SECONDS + 0.2), 1);
    }

    #[test]
    fn a_screen_that_needs_a_second_press_gets_one_soon_after() {
        let mut clock = IdleClock::default();
        assert_eq!(presses(&mut clock, IDLE_SECONDS + 0.2), 1);
        assert_eq!(presses(&mut clock, REPRESS_SECONDS + 0.2), 1);
    }

    #[test]
    fn a_key_or_a_new_screen_starts_the_wait_again() {
        let mut clock = IdleClock::default();
        assert_eq!(presses(&mut clock, IDLE_SECONDS - 0.5), 0);
        assert!(!clock.step(0.1, false));
        assert_eq!(presses(&mut clock, IDLE_SECONDS - 0.5), 0);
    }

    #[test]
    fn a_long_gap_between_frames_does_not_count_as_waiting() {
        let mut clock = IdleClock::default();
        assert!(!clock.step(60.0, true));
        assert!(clock.waited <= LONGEST_FRAME);
    }

    #[test]
    fn the_helm_and_a_pause_do_not_wait() {
        assert!(!screen_can_wait(GameState::Helm, PauseState::Running));
        assert!(!screen_can_wait(GameState::Boot, PauseState::Running));
        assert!(!screen_can_wait(GameState::Event, PauseState::Paused));
        assert!(screen_can_wait(GameState::Event, PauseState::Running));
        assert!(screen_can_wait(GameState::Title, PauseState::Running));
    }
}
