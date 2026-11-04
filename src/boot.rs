//! Loads runtime assets and prepares them, then opens the title screen.

use bevy::asset::{LoadState, RenderAssetUsages};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::content::{Content, ContentFiles, RonText, start_content_load};
use crate::states::GameState;
use crate::ui::centered;
use crate::ui::palette::{Pal, remap_rgba, rotate_rgba, silhouette_rgba};

#[derive(Resource, Clone)]
pub struct Sprites {
    /// The full-screen title card, drawn by `tools/gen_title_art.py`.
    pub title: Handle<Image>,
    pub side: Handle<Image>,
    pub topdown: Handle<Image>,
    /// A white shape of the side view, tinted with one palette color.
    pub silhouette: Handle<Image>,
    /// The upright ship and small turns either way, one 80x80 frame per angle
    /// in `HELM_ANGLES`.
    pub helm_frames: Handle<Image>,
}

/// Size of one frame of the top-down art.
pub const FRAME_SIZE: usize = 80;
/// Degrees clockwise for each column of `Sprites::helm_frames`.
pub const HELM_ANGLES: [f32; 5] = [-15.0, -8.0, 0.0, 8.0, 15.0];

/// Build the helm frames from the first frame of the top-down sheet, which
/// shows the ship upright. Returns `None` if the sheet has no pixel data.
fn helm_frames_image(sheet: &Image) -> Option<Image> {
    let data = sheet.data.as_ref()?;
    let stride = sheet.width() as usize * 4;
    let upright: Vec<u8> = (0..FRAME_SIZE)
        .flat_map(|row| {
            data[row * stride..row * stride + FRAME_SIZE * 4]
                .iter()
                .copied()
        })
        .collect();
    let turned: Vec<Vec<u8>> = HELM_ANGLES
        .iter()
        .map(|angle| rotate_rgba(&upright, FRAME_SIZE, *angle))
        .collect();
    let mut pixels = Vec::with_capacity(HELM_ANGLES.len() * FRAME_SIZE * FRAME_SIZE * 4);
    for row in 0..FRAME_SIZE {
        for frame in &turned {
            pixels.extend_from_slice(&frame[row * FRAME_SIZE * 4..(row + 1) * FRAME_SIZE * 4]);
        }
    }
    Some(Image::new(
        Extent3d {
            width: (HELM_ANGLES.len() * FRAME_SIZE) as u32,
            height: FRAME_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ))
}

/// The play page has already shown the title card and taken a key press to
/// allow sound, so the browser game doesn't ask for another one.
fn first_screen() -> GameState {
    if cfg!(target_arch = "wasm32") {
        GameState::Port
    } else {
        GameState::Title
    }
}

fn start_loading(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(Sprites {
        title: server.load("sprites/freighter/title_card.png"),
        side: server.load("sprites/freighter/freighter_side.png"),
        topdown: server.load("sprites/freighter/freighter_topdown_sheet.png"),
        silhouette: Handle::default(),
        helm_frames: Handle::default(),
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
    mut time: ResMut<Time<Virtual>>,
    mut failed: Local<bool>,
) {
    if *failed {
        return;
    }
    let handles = [
        sprites.title.clone(),
        sprites.side.clone(),
        sprites.topdown.clone(),
    ];
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
        Ok(content) => {
            time.set_relative_speed(content.balance.pace);
            commands.insert_resource(content);
        }
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
    let frames = images.get(&sprites.topdown).and_then(helm_frames_image);
    if let Some(frames) = frames {
        sprites.helm_frames = images.add(frames);
    }
    next.set(first_screen());
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
