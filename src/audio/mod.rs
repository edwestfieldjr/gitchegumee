//! Music layers driven by the storm, and sound effects.

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::states::{GameState, PauseState, UnderWay};
use crate::ui::Label;
use crate::ui::palette::Pal;
use crate::ui::pause::MuteHint;
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

/// Whether the player has turned the sound off. It lasts until they turn it
/// back on, across every screen.
#[derive(Resource, Debug, Default, PartialEq, Eq)]
pub struct Muted(pub bool);

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

fn title_music(server: &AssetServer, muted: &Muted) -> impl Bundle {
    (
        AudioPlayer::new(server.load("audio/title_theme.ogg")),
        PlaybackSettings {
            muted: muted.0,
            ..PlaybackSettings::LOOP.with_volume(Volume::Linear(TITLE_VOLUME))
        },
    )
}

fn start_title_music(mut commands: Commands, server: Res<AssetServer>, muted: Res<Muted>) {
    commands.spawn((
        title_music(&server, &muted),
        DespawnOnExit(GameState::Title),
    ));
}

/// The browser game opens at port, so the theme plays there the first time.
fn start_theme_in_first_port(
    mut commands: Commands,
    server: Res<AssetServer>,
    muted: Res<Muted>,
    mut played: Local<bool>,
) {
    if !std::mem::replace(&mut *played, true) {
        commands.spawn((title_music(&server, &muted), DespawnOnExit(GameState::Port)));
    }
}

fn start_layers(mut commands: Commands, server: Res<AssetServer>, muted: Res<Muted>) {
    for (kind, path) in [
        (LayerKind::Calm, "audio/music_calm.ogg"),
        (LayerKind::Storm, "audio/music_storm.ogg"),
        (LayerKind::Wind, "audio/music_wind.ogg"),
    ] {
        commands.spawn((
            MusicLayer(kind),
            AudioPlayer::new(server.load(path)),
            PlaybackSettings {
                muted: muted.0,
                ..PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0))
            },
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

fn hush_for_pause(sinks: Query<&AudioSink>) {
    for sink in &sinks {
        sink.pause();
    }
}

fn resume_after_pause(sinks: Query<&AudioSink>) {
    for sink in &sinks {
        sink.play();
    }
}

fn play_sfx(
    mut commands: Commands,
    server: Res<AssetServer>,
    muted: Res<Muted>,
    mut sfx: MessageReader<Sfx>,
) {
    for effect in sfx.read() {
        commands.spawn((
            AudioPlayer::new(server.load(effect.path())),
            PlaybackSettings {
                muted: muted.0,
                ..PlaybackSettings::DESPAWN.with_volume(Volume::Linear(SFX_VOLUME))
            },
        ));
    }
}

fn mute_input(keys: Res<ButtonInput<KeyCode>>, mut muted: ResMut<Muted>) {
    if keys.just_pressed(KeyCode::KeyM) {
        muted.0 = !muted.0;
    }
}

/// A muted sink keeps track of volume changes, so the storm's fades carry on
/// silently and the music comes back at the right level.
fn apply_mute(muted: Res<Muted>, mut sinks: Query<&mut AudioSink>) {
    for mut sink in &mut sinks {
        if sink.is_muted() != muted.0 {
            if muted.0 {
                sink.mute();
            } else {
                sink.unmute();
            }
        }
    }
}

/// The hint's words and color for whether the sound is off.
fn mute_hint(muted: bool) -> (&'static str, Pal) {
    if muted {
        ("M MUTED", Pal::Amber)
    } else {
        ("M MUTE", Pal::Gray)
    }
}

fn show_mute_hint(muted: Res<Muted>, mut hint: Single<&mut Label, With<MuteHint>>) {
    let (text, color) = mute_hint(muted.0);
    hint.set(text);
    if hint.color != color {
        hint.color = color;
    }
}

pub struct AudioGamePlugin;

impl Plugin for AudioGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Sfx>()
            .init_resource::<Muted>()
            .add_systems(OnEnter(GameState::Title), start_title_music)
            .add_systems(
                OnEnter(GameState::Port),
                start_theme_in_first_port.run_if(|| cfg!(target_arch = "wasm32")),
            )
            .add_systems(OnEnter(UnderWay), start_layers)
            .add_systems(OnEnter(PauseState::Paused), hush_for_pause)
            .add_systems(OnExit(PauseState::Paused), resume_after_pause)
            .add_systems(
                Update,
                (
                    (mute_input, play_sfx, apply_mute, show_mute_hint).chain(),
                    follow_storm.run_if(in_state(UnderWay)),
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mute_key_turns_the_sound_off_and_on() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Muted>()
            .add_systems(Update, mute_input);
        let press = |app: &mut App| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::KeyM);
            keys.clear();
            keys.press(KeyCode::KeyM);
            app.update();
        };

        assert_eq!(*app.world().resource::<Muted>(), Muted(false));
        press(&mut app);
        assert_eq!(*app.world().resource::<Muted>(), Muted(true));
        // Holding the key doesn't flip it back. Bevy's input system clears
        // "just pressed" each frame, and it isn't running in this test.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert_eq!(*app.world().resource::<Muted>(), Muted(true));
        press(&mut app);
        assert_eq!(*app.world().resource::<Muted>(), Muted(false));
    }

    #[test]
    fn the_hint_shows_when_the_sound_is_off() {
        assert_eq!(mute_hint(false), ("M MUTE", Pal::Gray));
        assert_eq!(mute_hint(true), ("M MUTED", Pal::Amber));
    }

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
