//! How the voyage ended, and the memorial.

mod arrival;
mod closing;
mod detroit;

use bevy::prelude::*;

use crate::audio::Sfx;
use crate::boot::Sprites;
use crate::content::{Content, clock_text};
use crate::rng::GameRng;
use crate::states::{GameState, MemorialPhase};
use crate::ui::font::wrap;
use crate::ui::palette::Pal;
use crate::ui::typing::{
    AfterHold, AfterTyping, FinishTyping, Typed, enter_may_advance, typed_lines,
};
use crate::ui::{advance_pressed, centered, label, rect, z};
use crate::voyage::{Ending, Voyage};
use closing::{CLOSING_SECONDS, ClosingCues, ClosingPart, ClosingWorld, SKIP_AFTER};
use detroit::{DetroitImage, belfry_light, build_detroit_image, scene_sprite};

const COLUMNS: usize = 50;
const TEXT_X: f32 = 10.0;
const CREW: u32 = 29;
const TOLL_SECONDS: f32 = 2.5;
/// How long the belfry stays lit after each stroke.
const BELFRY_LIT_SECONDS: f32 = 0.8;

#[derive(Resource, Default)]
pub(crate) struct OutcomeClock(f32);

fn enter_outcome(
    mut commands: Commands,
    ending: Res<Ending>,
    sprites: Res<Sprites>,
    detroit: Res<DetroitImage>,
    content: Res<Content>,
    server: Res<AssetServer>,
    mut rng: ResMut<GameRng>,
) {
    commands.insert_resource(OutcomeClock::default());
    if ending.reached_bay {
        arrival::spawn_arrival(&mut commands, &sprites, &detroit, &server);
    } else {
        closing::spawn_closing(
            &mut commands,
            &sprites,
            &content,
            &ending,
            &mut rng,
            &server,
        );
    }
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
    parts: Query<Entity, With<ClosingPart>>,
    mut sfx: MessageWriter<Sfx>,
    mut next: ResMut<NextState<GameState>>,
) {
    let words_at = if ending.reached_bay {
        arrival::ARRIVAL_SECONDS
    } else {
        CLOSING_SECONDS
    };
    let before = clock.0;
    clock.0 += time.delta_secs();
    if before < words_at && before >= SKIP_AFTER && advance_pressed(&keys) {
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
            AfterHold,
            Visibility::Hidden,
            scope,
        ));
    }
    if before >= words_at && advance_pressed(&keys) && enter_may_advance(&typed, &mut finish) {
        next.set(GameState::Memorial);
    }
}

#[derive(Resource, Default)]
struct Tolling {
    clock: f32,
    rung: u32,
    last_toll: f32,
}

#[derive(Component)]
struct BelfryLight;

fn enter_memorial_words(mut commands: Commands, content: Res<Content>) {
    let scope = DespawnOnExit(MemorialPhase::Words);
    commands.spawn((
        rect(0.0, 0.0, 320.0, 200.0, Pal::Black, z::BACKGROUND),
        scope.clone(),
    ));
    let text = &content.text;
    commands.spawn((
        centered(text.memorial_title.clone(), 8.0, Pal::White, 2.0),
        scope.clone(),
    ));
    for line in typed_lines(&text.memorial_body, COLUMNS, TEXT_X, 34.0, Pal::Foam) {
        commands.spawn((line, scope.clone()));
    }
    commands.spawn((
        centered("ENTER", 186.0, Pal::Gray, 1.0),
        AfterHold,
        Visibility::Hidden,
        scope,
    ));
}

fn memorial_words_input(
    keys: Res<ButtonInput<KeyCode>>,
    typed: Query<&Typed>,
    mut finish: MessageWriter<FinishTyping>,
    mut next: ResMut<NextState<MemorialPhase>>,
) {
    if advance_pressed(&keys) && enter_may_advance(&typed, &mut finish) {
        next.set(MemorialPhase::Bell);
    }
}

fn enter_memorial_bell(mut commands: Commands, content: Res<Content>, detroit: Res<DetroitImage>) {
    commands.insert_resource(Tolling::default());
    let scope = DespawnOnExit(MemorialPhase::Bell);
    commands.spawn((scene_sprite(&detroit), scope.clone()));
    for light in belfry_light() {
        commands.spawn((light, BelfryLight, Visibility::Hidden, scope.clone()));
    }
    commands.spawn((
        centered(
            content.text.memorial_bell_caption.clone(),
            6.0,
            Pal::Foam,
            1.0,
        ),
        scope.clone(),
    ));
    commands.spawn((centered("ENTER", 186.0, Pal::Gray, 1.0), scope));
}

/// How many times the bell has rung, `seconds` after it began.
fn tolls_by(seconds: f32) -> u32 {
    ((seconds / TOLL_SECONDS) as u32 + 1).min(CREW)
}

fn ring_bell(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut tolling: ResMut<Tolling>,
    mut lights: Query<&mut Visibility, With<BelfryLight>>,
    mut sfx: MessageWriter<Sfx>,
    mut next: ResMut<NextState<GameState>>,
) {
    tolling.clock += time.delta_secs();
    let due = tolls_by(tolling.clock);
    if due > tolling.rung {
        tolling.rung = due;
        tolling.last_toll = tolling.clock;
        sfx.write(Sfx::Bell);
    }
    let lit = tolling.clock - tolling.last_toll < BELFRY_LIT_SECONDS;
    for mut visibility in &mut lights {
        *visibility = if lit {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if advance_pressed(&keys) {
        next.set(GameState::Title);
    }
}

pub struct OutcomePlugin;

impl Plugin for OutcomePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, build_detroit_image)
            .add_systems(OnEnter(GameState::Outcome), enter_outcome)
            .add_systems(OnEnter(MemorialPhase::Words), enter_memorial_words)
            .add_systems(OnEnter(MemorialPhase::Bell), enter_memorial_bell)
            .add_systems(
                Update,
                (
                    (outcome_timeline, arrival::draw_arrival)
                        .chain()
                        .run_if(in_state(GameState::Outcome)),
                    memorial_words_input.run_if(in_state(MemorialPhase::Words)),
                    ring_bell.run_if(in_state(MemorialPhase::Bell)),
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
