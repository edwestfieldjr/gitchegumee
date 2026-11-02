//! The closing sequence on the counterfactual ending: the ship comes down the
//! Detroit River at dusk with her lights on.
//!
//! The skyline is a stand-in, to be redrawn from a reference photo. Every part
//! of it is original pixel art made of palette rectangles.

use bevy::audio::Volume;
use bevy::prelude::*;

use super::OutcomeClock;
use super::closing::ClosingPart;
use crate::audio::Sfx;
use crate::boot::Sprites;
use crate::content::Content;
use crate::states::GameState;
use crate::ui::palette::Pal;
use crate::ui::{at, centered, rect, z};

const HORIZON: f32 = 106.0;
const SHIP_Y: f32 = 122.0;
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

/// (left, width, height) of each building, from left to right.
const SKYLINE: [(f32, f32, f32); 14] = [
    (18.0, 14.0, 12.0),
    (34.0, 10.0, 20.0),
    (46.0, 16.0, 15.0),
    (66.0, 12.0, 30.0),
    (80.0, 18.0, 22.0),
    (100.0, 10.0, 36.0),
    (112.0, 14.0, 48.0),
    (128.0, 12.0, 34.0),
    (142.0, 18.0, 26.0),
    (162.0, 10.0, 40.0),
    (174.0, 16.0, 20.0),
    (196.0, 12.0, 28.0),
    (212.0, 20.0, 16.0),
    (238.0, 12.0, 22.0),
];
/// The tallest building carries a red light at its top.
const TOWER: usize = 6;

/// Where the ship is, easing to a stop.
pub fn ship_x(t: f32) -> f32 {
    let progress = (t / GLIDE_SECONDS).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - progress) * (1.0 - progress);
    SHIP_FROM + (SHIP_TO - SHIP_FROM) * eased
}

/// Whether a window is lit. Fixed for each window, so the city does not flicker.
fn window_lit(x: u32, y: u32) -> bool {
    (x.wrapping_mul(31) ^ y.wrapping_mul(17)).wrapping_mul(2_654_435_761) >> 30 == 0
}

#[derive(Component)]
pub(super) struct ArrivalShip;

#[derive(Component)]
pub(super) struct ArrivalCaption;

pub fn spawn_arrival(
    commands: &mut Commands,
    sprites: &Sprites,
    content: &Content,
    server: &AssetServer,
) {
    let scope = (DespawnOnExit(GameState::Outcome), ClosingPart);
    commands.spawn((
        rect(0.0, 0.0, 320.0, HORIZON, Pal::Night, z::BACKGROUND),
        scope.clone(),
    ));
    commands.spawn((
        rect(
            0.0,
            70.0,
            320.0,
            HORIZON - 70.0,
            Pal::Deep,
            z::BACKGROUND + 0.5,
        ),
        scope.clone(),
    ));
    commands.spawn((
        rect(
            0.0,
            HORIZON,
            320.0,
            200.0 - HORIZON,
            Pal::Deep,
            z::BACKGROUND,
        ),
        scope.clone(),
    ));
    commands.spawn((
        rect(0.0, HORIZON - 3.0, 320.0, 5.0, Pal::Shore, z::FIELD + 1.0),
        scope.clone(),
    ));
    for (x, y, w) in [
        (20.0, 136.0, 18.0),
        (70.0, 158.0, 24.0),
        (130.0, 148.0, 14.0),
        (196.0, 170.0, 26.0),
        (252.0, 142.0, 20.0),
        (290.0, 164.0, 16.0),
    ] {
        commands.spawn((rect(x, y, w, 1.0, Pal::Lake, z::FIELD), scope.clone()));
    }

    for (index, (left, width, height)) in SKYLINE.into_iter().enumerate() {
        let top = HORIZON - height;
        commands.spawn((
            rect(left, top, width, height, Pal::Black, z::FIELD),
            scope.clone(),
        ));
        let mut y = top + 2.0;
        while y < HORIZON - 4.0 {
            let mut x = left + 2.0;
            while x < left + width - 2.0 {
                if window_lit(x as u32, y as u32) {
                    commands.spawn((
                        rect(x, y, 1.0, 1.0, Pal::Amber, z::FIELD + 0.5),
                        scope.clone(),
                    ));
                }
                x += 3.0;
            }
            y += 3.0;
        }
        if index == TOWER {
            commands.spawn((
                rect(
                    left + width / 2.0 - 1.0,
                    top - 3.0,
                    2.0,
                    3.0,
                    Pal::Red,
                    z::FIELD,
                ),
                scope.clone(),
            ));
        }
    }

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
    commands.spawn((
        centered(content.text.arrival_caption.clone(), 14.0, Pal::Foam, 2.0),
        Visibility::Hidden,
        ArrivalCaption,
        scope.clone(),
    ));
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

    #[test]
    fn the_skyline_fits_the_view_above_the_river() {
        for (left, width, height) in SKYLINE {
            assert!(left >= 0.0 && left + width <= 320.0);
            assert!(HORIZON - height > 0.0);
        }
    }

    #[test]
    fn some_windows_are_lit_and_some_dark() {
        let lit = (0..50)
            .flat_map(|x| (0..50).map(move |y| window_lit(x, y)))
            .filter(|lit| *lit)
            .count();
        assert!(lit > 100 && lit < 2400, "{lit}");
    }
}
