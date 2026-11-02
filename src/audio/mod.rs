//! Music layers driven by the storm, and sound effects.

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::states::{GameState, UnderWay};
use crate::weather::Weather;

const TITLE_VOLUME: f32 = 0.6;
const LAYER_VOLUME: f32 = 0.5;
const SFX_VOLUME: f32 = 0.6;
/// How quickly layer volumes follow the storm, per second.
const FADE_RATE: f32 = 0.5;

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sfx {
    Select,
    Radio,
    Sea,
    Debris,
    Shoal,
    Rogue,
    Bell,
    Horn,
}

impl Sfx {
    fn path(self) -> &'static str {
        match self {
            Sfx::Select => "audio/sfx_select.ogg",
            Sfx::Radio => "audio/sfx_radio.ogg",
            Sfx::Sea => "audio/sfx_sea.ogg",
            Sfx::Debris => "audio/sfx_debris.ogg",
            Sfx::Shoal => "audio/sfx_shoal.ogg",
            Sfx::Rogue => "audio/sfx_rogue.ogg",
            Sfx::Bell => "audio/sfx_bell.ogg",
            Sfx::Horn => "audio/sfx_horn.ogg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayerKind {
    Calm,
    Storm,
    Wind,
}

#[derive(Component)]
struct MusicLayer(LayerKind);

/// Target volume of a layer at a storm strength from 0 to 1. The calm tune
/// gives way to the driving layer, and the wind comes in over the top.
fn layer_volume(kind: LayerKind, storm: f32) -> f32 {
    let storm = storm.clamp(0.0, 1.0);
    match kind {
        LayerKind::Calm => 1.0 - (storm * 2.5).min(1.0),
        LayerKind::Storm => ((storm - 0.2) * 2.0).clamp(0.0, 1.0),
        LayerKind::Wind => ((storm - 0.4) * 1.67).clamp(0.0, 1.0),
    }
}

fn start_title_music(mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn((
        AudioPlayer::new(server.load("audio/title_theme_placeholder.ogg")),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(TITLE_VOLUME)),
        DespawnOnExit(GameState::Title),
    ));
}

fn start_layers(mut commands: Commands, server: Res<AssetServer>) {
    for (kind, path) in [
        (LayerKind::Calm, "audio/music_calm.ogg"),
        (LayerKind::Storm, "audio/music_storm.ogg"),
        (LayerKind::Wind, "audio/music_wind.ogg"),
    ] {
        commands.spawn((
            MusicLayer(kind),
            AudioPlayer::new(server.load(path)),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
            DespawnOnExit(UnderWay),
        ));
    }
}

fn follow_storm(
    time: Res<Time>,
    weather: Res<Weather>,
    mut layers: Query<(&MusicLayer, &mut AudioSink)>,
) {
    let step = FADE_RATE * time.delta_secs();
    for (layer, mut sink) in &mut layers {
        let target = layer_volume(layer.0, weather.storm()) * LAYER_VOLUME;
        let current = sink.volume().to_linear();
        let next = current + (target - current).clamp(-step, step);
        sink.set_volume(Volume::Linear(next));
    }
}

fn play_sfx(mut commands: Commands, server: Res<AssetServer>, mut sfx: MessageReader<Sfx>) {
    for effect in sfx.read() {
        commands.spawn((
            AudioPlayer::new(server.load(effect.path())),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(SFX_VOLUME)),
        ));
    }
}

pub struct AudioGamePlugin;

impl Plugin for AudioGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Sfx>()
            .add_systems(OnEnter(GameState::Title), start_title_music)
            .add_systems(OnEnter(UnderWay), start_layers)
            .add_systems(Update, (play_sfx, follow_storm.run_if(in_state(UnderWay))));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calm_water_plays_only_the_calm_tune() {
        assert_eq!(layer_volume(LayerKind::Calm, 0.0), 1.0);
        assert_eq!(layer_volume(LayerKind::Storm, 0.0), 0.0);
        assert_eq!(layer_volume(LayerKind::Wind, 0.0), 0.0);
    }

    #[test]
    fn full_storm_drops_the_calm_tune() {
        assert_eq!(layer_volume(LayerKind::Calm, 1.0), 0.0);
        assert_eq!(layer_volume(LayerKind::Storm, 1.0), 1.0);
        assert_eq!(layer_volume(LayerKind::Wind, 1.0), 1.0);
    }

    #[test]
    fn the_layers_build_in_order_as_the_storm_rises() {
        let mut last = (1.0, 0.0, 0.0);
        for step in 0..=20 {
            let storm = step as f32 / 20.0;
            let now = (
                layer_volume(LayerKind::Calm, storm),
                layer_volume(LayerKind::Storm, storm),
                layer_volume(LayerKind::Wind, storm),
            );
            assert!(now.0 <= last.0 && now.1 >= last.1 && now.2 >= last.2);
            last = now;
        }
    }
}
