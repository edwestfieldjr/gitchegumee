//! Loads runtime assets and prepares them, then opens the title screen.

use bevy::asset::LoadState;
use bevy::prelude::*;

use crate::content::{Content, ContentFiles, RonText, start_content_load};
use crate::states::GameState;
use crate::ui::centered;
use crate::ui::palette::{Pal, remap_rgba, silhouette_rgba};

#[derive(Resource, Clone)]
pub struct Sprites {
    pub side: Handle<Image>,
    pub topdown: Handle<Image>,
    /// A white shape of the side view, tinted with one palette color.
    pub silhouette: Handle<Image>,
}

fn start_loading(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(Sprites {
        side: server.load("sprites/freighter/freighter_side.png"),
        topdown: server.load("sprites/freighter/freighter_topdown_sheet.png"),
        silhouette: Handle::default(),
    });
    commands.spawn((
        centered("LOADING", 96.0, Pal::Gray, 1.0),
        DespawnOnExit(GameState::Boot),
    ));
}

fn finish_loading(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut sprites: ResMut<Sprites>,
    files: Res<ContentFiles>,
    texts: Res<Assets<RonText>>,
    mut images: ResMut<Assets<Image>>,
    mut next: ResMut<NextState<GameState>>,
    mut failed: Local<bool>,
) {
    if *failed {
        return;
    }
    let handles = [sprites.side.clone(), sprites.topdown.clone()];
    if !handles
        .iter()
        .all(|handle| matches!(server.load_state(handle.id()), LoadState::Loaded))
    {
        return;
    }
    let loaded: Vec<&str> = files
        .0
        .iter()
        .filter_map(|handle| texts.get(handle).map(|text| text.0.as_str()))
        .collect();
    let Ok(sources) = <[&str; 6]>::try_from(loaded) else {
        return;
    };
    match Content::parse(sources) {
        Ok(content) => commands.insert_resource(content),
        Err(errors) => {
            *failed = true;
            for error in &errors {
                error!("content: {error}");
            }
            commands.spawn((
                centered("CONTENT ERROR - SEE LOG", 110.0, Pal::Red, 1.0),
                DespawnOnExit(GameState::Boot),
            ));
            return;
        }
    }
    for handle in &handles {
        if let Some(mut image) = images.get_mut(handle)
            && let Some(data) = image.data.as_mut()
        {
            remap_rgba(data);
        }
    }
    let shape = images.get(&sprites.side).map(|side| {
        let mut shape = side.clone();
        if let Some(data) = shape.data.as_mut() {
            silhouette_rgba(data);
        }
        shape
    });
    if let Some(shape) = shape {
        sprites.silhouette = images.add(shape);
    }
    next.set(GameState::Title);
}

pub struct BootPlugin;

impl Plugin for BootPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Boot),
            (start_loading, start_content_load),
        )
        .add_systems(Update, finish_loading.run_if(in_state(GameState::Boot)));
    }
}
