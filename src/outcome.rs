//! How the voyage ended, and the memorial.

use bevy::prelude::*;

use crate::audio::Sfx;
use crate::content::{Content, clock_text};
use crate::states::GameState;
use crate::ui::palette::Pal;
use crate::ui::{at, centered, label, rect, wrapped_lines, z};
use crate::voyage::map::{MAP_H, MAP_W, MapImage, project};
use crate::voyage::{Ending, Voyage};

const COLUMNS: usize = 50;
const TEXT_X: f32 = 10.0;
const MAP_X: f32 = 48.0;
const MAP_Y: f32 = 41.0;
/// Seconds the chart holds with the ship on it, before she is no longer shown.
const BLIP_SECONDS: f32 = 3.0;
/// Seconds before the words come up.
const TEXT_AFTER: f32 = 6.0;
const CREW: u32 = 29;
const TOLL_SECONDS: f32 = 2.5;

#[derive(Resource, Default)]
struct OutcomeClock(f32);

#[derive(Component)]
struct ChartPart;

#[derive(Component)]
struct ShipBlip;

fn enter_outcome(mut commands: Commands, ending: Res<Ending>, map: Res<MapImage>) {
    commands.insert_resource(OutcomeClock::default());
    let scope = DespawnOnExit(GameState::Outcome);
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
        ShipBlip,
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
    mut clock: ResMut<OutcomeClock>,
    blip: Query<Entity, With<ShipBlip>>,
    chart: Query<Entity, With<ChartPart>>,
    mut next: ResMut<NextState<GameState>>,
) {
    let before = clock.0;
    clock.0 += time.delta_secs();
    let crossed = |mark: f32| before < mark && clock.0 >= mark;

    // The ship simply stops being shown, unless she made it.
    if crossed(BLIP_SECONDS) && !ending.reached_bay {
        for entity in &blip {
            commands.entity(entity).despawn();
        }
    }
    if crossed(TEXT_AFTER) {
        for entity in chart.iter().chain(blip.iter()) {
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
        let lines = wrapped_lines(body, COLUMNS, 44.0);
        let mut bottom = 44.0;
        for (line, y) in lines {
            bottom = y;
            commands.spawn((label(line, TEXT_X, y, Pal::Foam), scope.clone()));
        }
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
                scope.clone(),
            ));
        }
        commands.spawn((
            label(
                format!(
                    "{:.0} MILES RUN.  STRUCK BY {} SEAS AND HAZARDS.",
                    voyage.nm, voyage.hazard_hits
                ),
                TEXT_X,
                166.0,
                Pal::Gray,
            ),
            scope.clone(),
        ));
        commands.spawn((centered("ENTER", 184.0, Pal::Amber, 1.0), scope));
    }
    if clock.0 >= TEXT_AFTER && keys.just_pressed(KeyCode::Enter) {
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
    for (line, y) in wrapped_lines(&text.memorial_body, COLUMNS, 30.0) {
        commands.spawn((label(line, TEXT_X, y, Pal::Foam), scope.clone()));
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
    commands.spawn((centered("ENTER", 186.0, Pal::Gray, 1.0), scope));
}

/// How many times the bell has rung, `seconds` after the memorial opened.
fn tolls_by(seconds: f32) -> u32 {
    ((seconds / TOLL_SECONDS) as u32 + 1).min(CREW)
}

fn ring_bell(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
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
    if keys.just_pressed(KeyCode::Enter) {
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
