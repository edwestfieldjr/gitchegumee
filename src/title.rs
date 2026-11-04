//! The title screen.

use bevy::prelude::*;

use crate::audio::Sfx;
use crate::boot::Sprites;
use crate::states::GameState;
use crate::ui::font::CELL_W;
use crate::ui::palette::Pal;
use crate::ui::{advance_pressed, at, centered, rect, z};

const PROMPT: &str = "PRESS ENTER OR TAP SCREEN";
const PROMPT_Y: f32 = 182.0;

fn enter_title(mut commands: Commands, sprites: Res<Sprites>) {
    let scope = DespawnOnExit(GameState::Title);
    commands.spawn((
        Sprite::from_image(sprites.title.clone()),
        at(160.0, 100.0, z::BACKGROUND),
        scope.clone(),
    ));
    let width = PROMPT.len() as f32 * CELL_W + 8.0;
    commands.spawn((
        rect(
            160.0 - width / 2.0,
            PROMPT_Y - 2.0,
            width,
            11.0,
            Pal::Black,
            z::PANEL,
        ),
        scope.clone(),
    ));
    commands.spawn((centered(PROMPT, PROMPT_Y, Pal::Yellow, 1.0), scope));
}

fn title_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut sfx: MessageWriter<Sfx>,
    mut next: ResMut<NextState<GameState>>,
) {
    if advance_pressed(&keys) {
        sfx.write(Sfx::Select);
        next.set(GameState::Port);
    }
}

pub struct TitlePlugin;

impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Title), enter_title)
            .add_systems(Update, title_input.run_if(in_state(GameState::Title)));
    }
}
