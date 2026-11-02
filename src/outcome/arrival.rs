//! The closing sequence on the counterfactual ending: the ship comes down the
//! river past Detroit at night with her lights on.

use bevy::audio::Volume;
use bevy::prelude::*;

use super::OutcomeClock;
use super::closing::ClosingPart;
use super::detroit::{DetroitImage, caption_pixel, caption_pixels, scene_sprite};
use crate::audio::Sfx;
use crate::boot::Sprites;
use crate::states::GameState;
use crate::ui::palette::Pal;
use crate::ui::{at, z};

const CAPTION: &str = "DETROIT";
const CAPTION_Y: f32 = 4.0;
const SHIP_Y: f32 = 136.0;
const SHIP_FROM: f32 = -60.0;
const SHIP_TO: f32 = 168.0;
const GLIDE_SECONDS: f32 = 22.0;
const HORN_AT: f32 = 20.0;
const CAPTION_AT: f32 = 2.0;
/// When the sequence is over and the words come up.
pub const ARRIVAL_SECONDS: f32 = 30.0;
const MUSIC_VOLUME: f32 = 0.45;

/// Lit parts of the ship, in sprite pixels.
const SHIP_LIGHTS: [(f32, f32, f32); 3] = [(80.0, 7.0, 8.0), (80.0, 12.0, 8.0), (7.0, 12.0, 14.0)];

/// Where the ship is, easing to a stop.
pub fn ship_x(t: f32) -> f32 {
    let progress = (t / GLIDE_SECONDS).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - progress) * (1.0 - progress);
    SHIP_FROM + (SHIP_TO - SHIP_FROM) * eased
}

#[derive(Component)]
pub(super) struct ArrivalShip;

#[derive(Component)]
pub(super) struct ArrivalCaption;

pub fn spawn_arrival(
    commands: &mut Commands,
    sprites: &Sprites,
    detroit: &DetroitImage,
    server: &AssetServer,
) {
    let scope = (DespawnOnExit(GameState::Outcome), ClosingPart);
    commands.spawn((scene_sprite(detroit), scope.clone()));

    commands
        .spawn((
            Sprite::from_image(sprites.side.clone()),
            at(ship_x(0.0), SHIP_Y, z::SHIP),
            ArrivalShip,
            scope.clone(),
        ))
        .with_children(|ship| {
            for (x, y, w) in SHIP_LIGHTS {
                ship.spawn((
                    Sprite {
                        color: Pal::Yellow.color(),
                        custom_size: Some(Vec2::new(w, 1.0)),
                        ..default()
                    },
                    Transform::from_xyz(x + w / 2.0 - 48.0, 15.0 - y - 0.5, 1.0),
                ));
            }
        });
    for (x, y) in caption_pixels(CAPTION, CAPTION_Y) {
        commands.spawn((
            caption_pixel(x, y),
            Visibility::Hidden,
            ArrivalCaption,
            scope.clone(),
        ));
    }
    commands.spawn((
        AudioPlayer::new(server.load("audio/music_calm.ogg")),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(MUSIC_VOLUME)),
        scope,
    ));
}

pub(super) fn draw_arrival(
    clock: Res<OutcomeClock>,
    mut last_t: Local<f32>,
    mut ship: Query<&mut Transform, With<ArrivalShip>>,
    mut caption: Query<&mut Visibility, With<ArrivalCaption>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let t = clock.0;
    if *last_t > t {
        *last_t = 0.0;
    }
    if *last_t < HORN_AT && t >= HORN_AT && !ship.is_empty() {
        sfx.write(Sfx::Horn);
    }
    *last_t = t;
    for mut transform in &mut ship {
        transform.translation.x = ship_x(t).round() - 160.0;
    }
    for mut visibility in &mut caption {
        *visibility = if t >= CAPTION_AT {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ship_comes_in_from_off_screen_and_stops() {
        assert!(ship_x(0.0) < -48.0, "she starts out of view");
        assert_eq!(ship_x(GLIDE_SECONDS), SHIP_TO);
        assert_eq!(ship_x(ARRIVAL_SECONDS), SHIP_TO);
    }

    #[test]
    fn the_ship_only_ever_moves_forward_and_slows_down() {
        let step = |t: f32| ship_x(t + 1.0) - ship_x(t);
        assert!(step(1.0) > step(15.0));
        assert!(step(15.0) > 0.0);
    }

    #[test]
    fn the_horn_sounds_during_the_sequence() {
        const { assert!(HORN_AT < ARRIVAL_SECONDS) };
    }
}
