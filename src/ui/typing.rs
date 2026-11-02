//! Teletype text, so that the player gets time to read.
//!
//! Text types out one character at a time. Enter finishes it, and only then
//! does the next Enter move on, so a panel cannot be skipped by accident.

use bevy::prelude::*;

use super::font::wrap;
use super::palette::Pal;
use super::{CELL_LINE, Label, at, z};

const CHARS_PER_SECOND: f32 = 28.0;
/// Pause at the end of each line, counted in characters.
const LINE_PAUSE_CHARS: usize = 6;

#[derive(Component, Debug, Clone)]
pub struct Typed {
    full: String,
    start: f32,
    elapsed: f32,
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
}

/// Stays hidden until all of the text on screen has finished typing.
#[derive(Component, Clone, Copy)]
pub struct AfterTyping;

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
                },
            )
        })
        .collect()
}

/// What Enter should do on a screen with typed text. Returns true when the
/// text is all out and the screen may move on; otherwise it finishes the text.
pub fn enter_may_advance(typed: &Query<&Typed>, finish: &mut MessageWriter<FinishTyping>) -> bool {
    if typed.iter().all(Typed::finished) {
        true
    } else {
        finish.write(FinishTyping);
        false
    }
}

fn advance_typing(
    time: Res<Time>,
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
        let shown: String = line.full.chars().take(line.shown_chars()).collect();
        label.set(shown);
    }
}

fn reveal_after_typing(
    typed: Query<&Typed>,
    mut waiting: Query<&mut Visibility, With<AfterTyping>>,
) {
    let done = typed.iter().all(Typed::finished);
    for mut visibility in &mut waiting {
        *visibility = if done {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
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
    fn lines_type_in_order_from_the_top() {
        let bundles = typed_lines("ONE LINE\nTWO LINE\nTHREE", 20, 0.0, 0.0, Pal::White);
        let starts: Vec<f32> = bundles.iter().map(|bundle| bundle.2.start).collect();
        assert_eq!(starts.len(), 3);
        assert!(starts[0] < starts[1] && starts[1] < starts[2]);
    }
}
