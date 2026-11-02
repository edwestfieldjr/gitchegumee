//! Loads runtime assets and prepares them, then opens the title screen.

use bevy::asset::LoadState;
use bevy::prelude::*;

use crate::states::GameState;
use crate::ui::palette::{Pal, remap_rgba};
use crate::ui::{centered, rect, z};

#[derive(Resource, Clone)]
pub struct Sprites {
    pub side: Handle<Image>,
    pub topdown: Handle<Image>,
}

fn start_loading(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(Sprites {
        side: server.load("sprites/freighter/freighter_side.png"),
        topdown: server.load("sprites/freighter/freighter_topdown_sheet.png"),
    });
    commands.spawn((
        centered("LOADING", 96.0, Pal::Gray, 1.0),
        DespawnOnExit(GameState::Boot),
    ));
}

fn finish_loading(
    server: Res<AssetServer>,
    sprites: Res<Sprites>,
    mut images: ResMut<Assets<Image>>,
    mut next: ResMut<NextState<GameState>>,
) {
    let handles = [&sprites.side, &sprites.topdown];
    if !handles
        .iter()
        .all(|handle| matches!(server.load_state(handle.id()), LoadState::Loaded))
    {
        return;
    }
    for handle in handles {
        if let Some(mut image) = images.get_mut(handle)
            && let Some(data) = image.data.as_mut()
        {
            remap_rgba(data);
        }
    }
    next.set(GameState::Title);
}

fn enter_title(mut commands: Commands, sprites: Res<Sprites>) {
    let scope = DespawnOnExit(GameState::Title);
    commands.spawn((
        rect(0.0, 0.0, 320.0, 200.0, Pal::Night, z::BACKGROUND),
        scope.clone(),
    ));
    commands.spawn((
        centered("GITCHE GUMEE", 32.0, Pal::White, 3.0),
        scope.clone(),
    ));
    commands.spawn((
        centered(
            "THE LAST VOYAGE OF THE EDMUND FITZGERALD",
            62.0,
            Pal::Foam,
            1.0,
        ),
        scope.clone(),
    ));
    commands.spawn((
        Sprite::from_image(sprites.side.clone()),
        crate::ui::at(160.0, 110.0, z::SHIP),
        scope,
    ));
}

pub struct BootPlugin;

impl Plugin for BootPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Boot), start_loading)
            .add_systems(Update, finish_loading.run_if(in_state(GameState::Boot)))
            .add_systems(OnEnter(GameState::Title), enter_title);
    }
}
