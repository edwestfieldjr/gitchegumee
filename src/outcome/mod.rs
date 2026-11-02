//! How the voyage ended, and the memorial.

mod closing;

use bevy::prelude::*;

use crate::audio::Sfx;
use crate::boot::Sprites;
use crate::content::{Content, clock_text};
use crate::rng::GameRng;
use crate::states::GameState;
use crate::ui::font::wrap;
use crate::ui::palette::Pal;
use crate::ui::typing::{AfterTyping, FinishTyping, Typed, enter_may_advance, typed_lines};
use crate::ui::{at, centered, label, rect, z};
use crate::voyage::map::{MAP_H, MAP_W, MapImage, project};
use crate::voyage::{Ending, Voyage};
use closing::{CLOSING_SECONDS, ClosingCues, ClosingPart, ClosingWorld, SKIP_AFTER};

const COLUMNS: usize = 50;
const TEXT_X: f32 = 10.0;
const MAP_X: f32 = 48.0;
const MAP_Y: f32 = 41.0;
/// Seconds before the words come up.
const TEXT_AFTER: f32 = 4.0;
const CREW: u32 = 29;
const TOLL_SECONDS: f32 = 2.5;

#[derive(Resource, Default)]
struct OutcomeClock(f32);

#[derive(Component)]
struct ChartPart;

fn enter_outcome(
    mut commands: Commands,
    ending: Res<Ending>,
    map: Res<MapImage>,
    sprites: Res<Sprites>,
    content: Res<Content>,
    server: Res<AssetServer>,
    mut rng: ResMut<GameRng>,
) {
    commands.insert_resource(OutcomeClock::default());
    let scope = DespawnOnExit(GameState::Outcome);
    if !ending.reached_bay {
        closing::spawn_closing(
            &mut commands,
            &sprites,
            &content,
            &ending,
            &mut rng,
            &server,
        );
        return;
    }
    commands.spawn((
        rect(0.0, 0.0, 320.0, 200.0, Pal::Black, z::BACKGROUND),
        scope.clone(),
    ));
    commands.spawn((
        Sprite::from_image(map.0.clone()),
        at(
            MAP_X + MAP_W as f32 / 2.0,
            MAP_Y + MAP_H as f32 / 2.0,
            z::FIELD,
        ),
        ChartPart,
        scope.clone(),
    ));
    let spot = project(ending.position);
    commands.spawn((
        rect(
            MAP_X + spot.x.floor() - 1.0,
            MAP_Y + spot.y.floor() - 1.0,
            3.0,
            3.0,
            Pal::Yellow,
            z::SHIP,
        ),
        ChartPart,
        scope,
    ));
}

fn outcome_timeline(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    ending: Res<Ending>,
    voyage: Res<Voyage>,
    content: Res<Content>,
    sprites: Res<Sprites>,
    typed: Query<&Typed>,
    mut finish: MessageWriter<FinishTyping>,
    mut clock: ResMut<OutcomeClock>,
    mut cues: Option<ResMut<ClosingCues>>,
    mut world: ClosingWorld,
    parts: Query<Entity, Or<(With<ChartPart>, With<ClosingPart>)>>,
    mut sfx: MessageWriter<Sfx>,
    mut next: ResMut<NextState<GameState>>,
) {
    let words_at = if ending.reached_bay {
        TEXT_AFTER
    } else {
        CLOSING_SECONDS
    };
    let before = clock.0;
    clock.0 += time.delta_secs();
    if !ending.reached_bay
        && before < words_at
        && before >= SKIP_AFTER
        && keys.just_pressed(KeyCode::Enter)
    {
        clock.0 = words_at;
    }
    let reached_words = before < words_at && clock.0 >= words_at;

    if clock.0 < words_at
        && let Some(cues) = cues.as_deref_mut()
    {
        closing::draw_closing(clock.0, &time, cues, &sprites, &mut sfx, &mut world);
    }
    if reached_words {
        for entity in &parts {
            commands.entity(entity).try_despawn();
        }
        let scope = DespawnOnExit(GameState::Outcome);
        let text = &content.text;
        let (title, body) = if ending.reached_bay {
            (&text.reached_title, &text.reached_body)
        } else {
            (&text.lost_title, &text.lost_body)
        };
        commands.spawn((
            centered(title.clone(), 16.0, Pal::White, 2.0),
            scope.clone(),
        ));
        for line in typed_lines(body, COLUMNS, TEXT_X, 44.0, Pal::Foam) {
            commands.spawn((line, scope.clone()));
        }
        let bottom = 44.0 + (wrap(body, COLUMNS).len() - 1) as f32 * 8.0;
        let gate = content.balance.last_contact_gate.minute();
        if !ending.reached_bay && ending.minute < gate {
            commands.spawn((
                label(
                    format!(
                        "ON THIS VOYAGE, THE SHIP WAS LOST AT {}.",
                        clock_text(ending.minute)
                    ),
                    TEXT_X,
                    bottom + 14.0,
                    Pal::Gray,
                ),
                AfterTyping,
                Visibility::Hidden,
                scope.clone(),
            ));
        }
        commands.spawn((
            label(
                format!(
                    "{:.0} MILES RUN.  STRUCK BY {} SEAS AND HAZARDS.",
                    voyage.run_nm, voyage.hazard_hits
                ),
                TEXT_X,
                166.0,
                Pal::Gray,
            ),
            AfterTyping,
            Visibility::Hidden,
            scope.clone(),
        ));
        commands.spawn((
            centered("ENTER", 184.0, Pal::Amber, 1.0),
            AfterTyping,
            Visibility::Hidden,
            scope,
        ));
    }
    if before >= words_at
        && keys.just_pressed(KeyCode::Enter)
        && enter_may_advance(&typed, &mut finish)
    {
        next.set(GameState::Memorial);
    }
}

#[derive(Resource, Default)]
struct Tolling {
    clock: f32,
    rung: u32,
}

#[derive(Component)]
struct TollMark(u32);

fn enter_memorial(mut commands: Commands, content: Res<Content>) {
    commands.insert_resource(Tolling::default());
    let scope = DespawnOnExit(GameState::Memorial);
    commands.spawn((
        rect(0.0, 0.0, 320.0, 200.0, Pal::Black, z::BACKGROUND),
        scope.clone(),
    ));
    let text = &content.text;
    commands.spawn((
        centered(text.memorial_title.clone(), 8.0, Pal::White, 2.0),
        scope.clone(),
    ));
    for line in typed_lines(&text.memorial_body, COLUMNS, TEXT_X, 30.0, Pal::Foam) {
        commands.spawn((line, scope.clone()));
    }
    let row_width = CREW as f32 * 8.0;
    let left = ((320.0 - row_width) / 2.0).floor();
    for mark in 0..CREW {
        commands.spawn((
            rect(
                left + mark as f32 * 8.0 + 2.0,
                168.0,
                4.0,
                6.0,
                Pal::Slate,
                z::HUD,
            ),
            TollMark(mark),
            scope.clone(),
        ));
    }
    commands.spawn((
        centered("ENTER", 186.0, Pal::Gray, 1.0),
        AfterTyping,
        Visibility::Hidden,
        scope,
    ));
}

/// How many times the bell has rung, `seconds` after the memorial opened.
fn tolls_by(seconds: f32) -> u32 {
    ((seconds / TOLL_SECONDS) as u32 + 1).min(CREW)
}

fn ring_bell(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    typed: Query<&Typed>,
    mut finish: MessageWriter<FinishTyping>,
    mut tolling: ResMut<Tolling>,
    mut marks: Query<(&TollMark, &mut Sprite)>,
    mut sfx: MessageWriter<Sfx>,
    mut next: ResMut<NextState<GameState>>,
) {
    tolling.clock += time.delta_secs();
    let due = tolls_by(tolling.clock);
    if due > tolling.rung {
        tolling.rung = due;
        sfx.write(Sfx::Bell);
        for (mark, mut sprite) in &mut marks {
            if mark.0 < due {
                sprite.color = Pal::Yellow.color();
            }
        }
    }
    if keys.just_pressed(KeyCode::Enter) && enter_may_advance(&typed, &mut finish) {
        next.set(GameState::Title);
    }
}

pub struct OutcomePlugin;

impl Plugin for OutcomePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Outcome), enter_outcome)
            .add_systems(OnEnter(GameState::Memorial), enter_memorial)
            .add_systems(
                Update,
                (
                    outcome_timeline.run_if(in_state(GameState::Outcome)),
                    ring_bell.run_if(in_state(GameState::Memorial)),
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bell_starts_at_once_and_stops_at_twenty_nine() {
        assert_eq!(tolls_by(0.0), 1);
        assert_eq!(tolls_by(TOLL_SECONDS), 2);
        assert_eq!(tolls_by(10_000.0), 29);
    }
}
