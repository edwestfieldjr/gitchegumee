//! The title screen.

use bevy::prelude::*;

use crate::audio::Sfx;
use crate::boot::Sprites;
use crate::states::GameState;
use crate::ui::palette::Pal;
use crate::ui::{at, centered, rect, z};

const WATERLINE: f32 = 128.0;
const SHIP_Y: f32 = 113.0;

fn enter_title(mut commands: Commands, sprites: Res<Sprites>) {
    let scope = DespawnOnExit(GameState::Title);
    commands.spawn((
        rect(0.0, 0.0, 320.0, WATERLINE, Pal::Night, z::BACKGROUND),
        scope.clone(),
    ));
    commands.spawn((
        rect(0.0, WATERLINE, 320.0, 72.0, Pal::Deep, z::BACKGROUND),
        scope.clone(),
    ));
    for (x, y, width) in [
        (18.0, 134.0, 22.0),
        (74.0, 141.0, 14.0),
        (212.0, 136.0, 26.0),
        (276.0, 146.0, 18.0),
        (132.0, 150.0, 20.0),
    ] {
        commands.spawn((rect(x, y, width, 1.0, Pal::Lake, z::FIELD), scope.clone()));
    }
    commands.spawn((
        centered("GITCHE GUMEE", 22.0, Pal::White, 3.0),
        scope.clone(),
    ));
    commands.spawn((
        centered(
            "THE LAST VOYAGE OF THE EDMUND FITZGERALD",
            52.0,
            Pal::Foam,
            1.0,
        ),
        scope.clone(),
    ));
    commands.spawn((
        centered("LAKE SUPERIOR  NOVEMBER 9-10, 1975", 66.0, Pal::Gray, 1.0),
        scope.clone(),
    ));
    commands.spawn((
        Sprite::from_image(sprites.side.clone()),
        at(160.0, SHIP_Y, z::SHIP),
        scope.clone(),
    ));
    commands.spawn((centered("PRESS ENTER", 182.0, Pal::Amber, 1.0), scope));
}

fn title_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut sfx: MessageWriter<Sfx>,
    mut next: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Enter) {
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
