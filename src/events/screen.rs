//! The boxed panel for an event: read, choose, read the result.

use bevy::prelude::*;

use super::{ActiveEvent, ApplyEffect};
use crate::audio::Sfx;
use crate::content::{Content, PANEL_COLUMNS, clock_text};
use crate::states::{EventPhase, GameState};
use crate::ui::palette::Pal;
use crate::ui::typing::{
    AfterHold, AfterTyping, FinishTyping, Typed, enter_may_advance, typed_lines,
};
use crate::ui::{Label, advance_pressed, label, panel_parts, rect, z};
use crate::voyage::Voyage;

const PANEL_X: f32 = 4.0;
const PANEL_W: f32 = 312.0;
const TEXT_X: f32 = 16.0;
const CHOICE_Y: f32 = 136.0;
const HINT_Y: f32 = 180.0;

#[derive(Resource, Debug, Default)]
struct Selection {
    choice: usize,
    picked: Option<usize>,
}

#[derive(Component)]
struct ChoiceRow(usize);

fn panel_backdrop(commands: &mut Commands, scope: impl Bundle + Clone) {
    commands.spawn((
        rect(0.0, 0.0, 320.0, 200.0, Pal::Black, z::BACKGROUND),
        scope.clone(),
    ));
    let [frame, inner] = panel_parts(PANEL_X, 4.0, PANEL_W, 192.0);
    commands.spawn((frame, scope.clone()));
    commands.spawn((inner, scope));
}

fn enter_choosing(
    mut commands: Commands,
    active: Res<ActiveEvent>,
    content: Res<Content>,
    voyage: Res<Voyage>,
) {
    commands.insert_resource(Selection::default());
    let event = &content.events[active.0];
    let scope = DespawnOnExit(EventPhase::Choosing);
    panel_backdrop(&mut commands, scope.clone());
    commands.spawn((
        label(event.title.clone(), TEXT_X, 12.0, Pal::Amber),
        scope.clone(),
    ));
    let clock = clock_text(voyage.minute);
    let clock_x = PANEL_X + PANEL_W - 10.0 - clock.len() as f32 * 6.0;
    commands.spawn((label(clock, clock_x, 12.0, Pal::Gray), scope.clone()));
    for line in typed_lines(&event.body, PANEL_COLUMNS, TEXT_X, 28.0, Pal::White) {
        commands.spawn((line, scope.clone()));
    }
    for (index, choice) in event.choices.iter().enumerate() {
        commands.spawn((
            label(
                choice.label.clone(),
                TEXT_X,
                CHOICE_Y + index as f32 * 12.0,
                Pal::Foam,
            ),
            ChoiceRow(index),
            AfterTyping,
            Visibility::Hidden,
            scope.clone(),
        ));
    }
    let hint = if event.choices.is_empty() {
        "ENTER: CONTINUE"
    } else {
        "UP/DOWN: CHOOSE   ENTER: GIVE THE ORDER"
    };
    commands.spawn((
        label(hint, TEXT_X, HINT_Y, Pal::Gray),
        AfterHold,
        Visibility::Hidden,
        scope,
    ));
}

fn choosing_input(
    keys: Res<ButtonInput<KeyCode>>,
    active: Res<ActiveEvent>,
    content: Res<Content>,
    typed: Query<&Typed>,
    mut finish: MessageWriter<FinishTyping>,
    mut selection: ResMut<Selection>,
    mut effects: MessageWriter<ApplyEffect>,
    mut sfx: MessageWriter<Sfx>,
    mut phase: ResMut<NextState<EventPhase>>,
    mut game: ResMut<NextState<GameState>>,
) {
    let event = &content.events[active.0];
    let count = event.choices.len();
    if count > 0 {
        if keys.just_pressed(KeyCode::ArrowDown) {
            selection.choice = (selection.choice + 1) % count;
            sfx.write(Sfx::Select);
        }
        if keys.just_pressed(KeyCode::ArrowUp) {
            selection.choice = (selection.choice + count - 1) % count;
            sfx.write(Sfx::Select);
        }
    }
    if !advance_pressed(&keys) || !enter_may_advance(&typed, &mut finish) {
        return;
    }
    sfx.write(Sfx::Select);
    match event.choices.get(selection.choice) {
        Some(choice) => {
            for effect in &choice.effects {
                effects.write(ApplyEffect(effect.clone()));
            }
            selection.picked = Some(selection.choice);
            phase.set(EventPhase::Result);
        }
        None => game.set(GameState::Voyage),
    }
}

fn draw_choices(selection: Res<Selection>, mut rows: Query<(&ChoiceRow, &mut Label)>) {
    for (row, mut text) in &mut rows {
        let selected = row.0 == selection.choice;
        let wanted = if selected { Pal::Yellow } else { Pal::Foam };
        if text.color != wanted {
            text.color = wanted;
        }
        let body = text.text.trim_start_matches("> ").trim_start().to_string();
        let shown = if selected {
            format!("> {body}")
        } else {
            format!("  {body}")
        };
        text.set(shown);
    }
}

fn enter_result(
    mut commands: Commands,
    active: Res<ActiveEvent>,
    selection: Res<Selection>,
    content: Res<Content>,
) {
    let event = &content.events[active.0];
    let scope = DespawnOnExit(EventPhase::Result);
    panel_backdrop(&mut commands, scope.clone());
    commands.spawn((
        label(event.title.clone(), TEXT_X, 12.0, Pal::Amber),
        scope.clone(),
    ));
    if let Some(choice) = selection.picked.and_then(|index| event.choices.get(index)) {
        for line in typed_lines(&choice.result, PANEL_COLUMNS, TEXT_X, 28.0, Pal::White) {
            commands.spawn((line, scope.clone()));
        }
    }
    commands.spawn((
        label("ENTER: CONTINUE", TEXT_X, HINT_Y, Pal::Gray),
        AfterHold,
        Visibility::Hidden,
        scope,
    ));
}

fn result_input(
    keys: Res<ButtonInput<KeyCode>>,
    typed: Query<&Typed>,
    mut finish: MessageWriter<FinishTyping>,
    mut sfx: MessageWriter<Sfx>,
    mut game: ResMut<NextState<GameState>>,
) {
    if advance_pressed(&keys) && enter_may_advance(&typed, &mut finish) {
        sfx.write(Sfx::Select);
        game.set(GameState::Voyage);
    }
}

pub struct EventScreenPlugin;

impl Plugin for EventScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(EventPhase::Choosing), enter_choosing)
            .add_systems(OnEnter(EventPhase::Result), enter_result)
            .add_systems(
                Update,
                (choosing_input, draw_choices)
                    .chain()
                    .run_if(in_state(EventPhase::Choosing)),
            )
            .add_systems(Update, result_input.run_if(in_state(EventPhase::Result)));
    }
}
