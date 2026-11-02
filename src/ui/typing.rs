//! Teletype text, so that the player gets time to read.
//!
//! Text types out one character at a time. Space or Enter finishes it, and the
//! screen stays until another press moves on, so the player sets the reading
//! time. [`HOLD_SECONDS`] keeps a double press from skipping the text.

use bevy::prelude::*;

use super::font::wrap;
use super::palette::Pal;
use super::{CELL_LINE, Label, at, z};

/// In game time, so at a `pace` of 0.4375 this is 17.5 characters a real second.
const CHARS_PER_SECOND: f32 = 40.0;
/// Real seconds after text finishes before a press can move on.
const HOLD_SECONDS: f32 = 0.5;
/// Pause at the end of each line, counted in characters.
const LINE_PAUSE_CHARS: usize = 6;

#[derive(Component, Debug, Clone)]
pub struct Typed {
    full: String,
    start: f32,
    elapsed: f32,
    held: f32,
}

impl Typed {
    /// How many characters show at this point in time.
    pub fn shown_chars(&self) -> usize {
        let typed = (self.elapsed - self.start).max(0.0) * CHARS_PER_SECOND;
        (typed as usize).min(self.full.len())
    }

    pub fn finished(&self) -> bool {
        self.shown_chars() >= self.full.len()
    }

    /// Finished, and past the guard against a double press.
    pub fn settled(&self) -> bool {
        self.finished() && self.held >= HOLD_SECONDS
    }
}

/// Stays hidden until all of the text on screen has finished typing.
#[derive(Component, Clone, Copy)]
pub struct AfterTyping;

/// Stays hidden until a press can move on from the screen.
#[derive(Component, Clone, Copy)]
pub struct AfterHold;

/// Ask for all the text on screen to finish at once.
#[derive(Message, Debug, Clone, Copy)]
pub struct FinishTyping;

/// Bundles for wrapped text that types out from top to bottom.
pub fn typed_lines(
    text: &str,
    columns: usize,
    x: f32,
    y: f32,
    color: Pal,
) -> Vec<(Label, Transform, Typed)> {
    let mut delay_chars = 0;
    wrap(text, columns)
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            let start = delay_chars as f32 / CHARS_PER_SECOND;
            delay_chars += line.len() + LINE_PAUSE_CHARS;
            (
                Label::new("", color),
                at(x, y + index as f32 * CELL_LINE, z::TEXT),
                Typed {
                    full: line,
                    start,
                    elapsed: 0.0,
                    held: 0.0,
                },
            )
        })
        .collect()
}

/// What a press should do on a screen with typed text. Returns true when the
/// text has sat long enough for the screen to move on; otherwise it finishes
/// any text still typing.
pub fn enter_may_advance(typed: &Query<&Typed>, finish: &mut MessageWriter<FinishTyping>) -> bool {
    if typed.iter().all(Typed::settled) {
        true
    } else {
        finish.write(FinishTyping);
        false
    }
}

fn advance_typing(
    time: Res<Time>,
    real: Res<Time<Real>>,
    mut finish: MessageReader<FinishTyping>,
    mut typed: Query<(&mut Typed, &mut Label)>,
) {
    let skip = finish.read().count() > 0;
    for (mut line, mut label) in &mut typed {
        if skip {
            line.elapsed = f32::MAX;
        } else if !line.finished() {
            line.elapsed += time.delta_secs();
        }
        if line.finished() {
            line.held += real.delta_secs();
        }
        let shown: String = line.full.chars().take(line.shown_chars()).collect();
        label.set(shown);
    }
}

fn shown_when(done: bool) -> Visibility {
    if done {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

fn reveal_after_typing(
    typed: Query<&Typed>,
    mut after_typing: Query<&mut Visibility, (With<AfterTyping>, Without<AfterHold>)>,
    mut after_hold: Query<&mut Visibility, (With<AfterHold>, Without<AfterTyping>)>,
) {
    let finished = typed.iter().all(Typed::finished);
    for mut visibility in &mut after_typing {
        *visibility = shown_when(finished);
    }
    let settled = typed.iter().all(Typed::settled);
    for mut visibility in &mut after_hold {
        *visibility = shown_when(settled);
    }
}

pub struct TypingPlugin;

impl Plugin for TypingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FinishTyping>()
            .add_systems(Update, (advance_typing, reveal_after_typing).chain());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(full: &str, start: f32, elapsed: f32) -> Typed {
        Typed {
            full: full.to_string(),
            start,
            elapsed,
            held: 0.0,
        }
    }

    #[test]
    fn nothing_shows_before_its_turn() {
        assert_eq!(line("HELLO", 1.0, 0.5).shown_chars(), 0);
    }

    #[test]
    fn characters_come_out_at_a_readable_rate() {
        let half_second = (CHARS_PER_SECOND * 0.5) as usize;
        let long = "X".repeat(100);
        assert_eq!(line(&long, 0.0, 0.5).shown_chars(), half_second);
        assert!((10.0..=40.0).contains(&CHARS_PER_SECOND));
    }

    #[test]
    fn a_finished_line_stops_at_its_length() {
        let done = line("HELLO", 0.0, 1000.0);
        assert_eq!(done.shown_chars(), 5);
        assert!(done.finished());
    }

    #[test]
    fn skipping_finishes_a_line_that_has_not_started() {
        let skipped = line("HELLO", 50.0, f32::MAX);
        assert!(skipped.finished());
    }

    #[test]
    fn finished_text_must_sit_before_the_screen_can_leave() {
        let mut done = line("HELLO", 0.0, 1000.0);
        assert!(done.finished() && !done.settled());
        done.held = HOLD_SECONDS - 0.01;
        assert!(!done.settled());
        done.held = HOLD_SECONDS;
        assert!(done.settled());
    }

    #[test]
    fn text_still_typing_never_counts_as_settled() {
        let mut typing = line("HELLO", 50.0, 0.0);
        typing.held = HOLD_SECONDS;
        assert!(!typing.settled());
    }

    #[test]
    fn lines_type_in_order_from_the_top() {
        let bundles = typed_lines("ONE LINE\nTWO LINE\nTHREE", 20, 0.0, 0.0, Pal::White);
        let starts: Vec<f32> = bundles.iter().map(|bundle| bundle.2.start).collect();
        assert_eq!(starts.len(), 3);
        assert!(starts[0] < starts[1] && starts[1] < starts[2]);
    }
}
