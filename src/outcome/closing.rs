//! The closing sequence on the historical ending: the ship in the storm, her
//! lights going out, an empty sea, then the wreck on the bottom.
//!
//! What sank the ship is unknown, so this never shows how she was lost. She
//! goes dark, the sea rises over the whole view, and then she is not there.

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::audio::Sfx;
use crate::boot::Sprites;
use crate::content::{Content, clock_text};
use crate::rng::GameRng;
use crate::states::GameState;
use crate::ui::palette::Pal;
use crate::ui::{Label, at, centered, rect, z};
use crate::voyage::Ending;

/// Seconds at which each beat begins.
const SEAS_AT: [f32; 3] = [3.0, 6.5, 10.0];
const ALL_DARK_AT: f32 = 13.0;
const COVERED_AT: f32 = 18.0;
const COVER_RISE: f32 = 3.0;
const GONE_TEXT_AT: f32 = 20.5;
const WIND_FADE_FROM: f32 = 19.5;
const BLACK_AT: f32 = 24.0;
const DESCENT_AT: f32 = 26.0;
const WRECK_AT: f32 = 35.0;
/// When the closing sequence is over and the words come up.
pub const CLOSING_SECONDS: f32 = 43.0;
/// Seconds before Enter may skip the sequence.
pub const SKIP_AFTER: f32 = 4.0;

const WATERLINE: f32 = 111.0;
const SHIP_W: f32 = 96.0;
const SHIP_H: f32 = 30.0;
const SHIP_X: f32 = 160.0;
const SHIP_Y: f32 = 101.0;
const SHIP_LEFT: f32 = SHIP_X - SHIP_W / 2.0;
const SHIP_TOP: f32 = SHIP_Y - SHIP_H / 2.0;
const COLUMN_W: f32 = 8.0;
const COLUMNS: usize = 40;
const SEA_AMPLITUDE: f32 = 5.0;
const SWELL_PEAK: [f32; 3] = [12.0, 18.0, 24.0];
const SWELL_WIDTH: f32 = 46.0;
const SWELL_SPEED: f32 = 70.0;
/// How high the sea stands at the moment it covers the view.
const COVER_HEIGHT: f32 = 96.0;
const FLAKES: usize = 45;
const WIND_VOLUME: f32 = 0.5;

const WRECK_DEPTH_FT: f32 = 530.0;
const FLOOR_Y: f32 = 164.0;
const FLOOR_RISE_FROM: f32 = 0.72;
/// Width of each section cut from the side sprite.
const SECTION_W: f32 = 40.0;

/// One lit part of the ship, in sprite pixels, and when it goes out.
struct ShipLight {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Pal,
    out_at: f32,
}

const LIGHTS: [ShipLight; 6] = [
    ShipLight {
        x: 84.0,
        y: 1.0,
        w: 1.0,
        h: 1.0,
        color: Pal::White,
        out_at: 9.6,
    },
    ShipLight {
        x: 80.0,
        y: 7.0,
        w: 8.0,
        h: 1.0,
        color: Pal::Yellow,
        out_at: 10.8,
    },
    ShipLight {
        x: 80.0,
        y: 12.0,
        w: 8.0,
        h: 1.0,
        color: Pal::Yellow,
        out_at: 11.3,
    },
    ShipLight {
        x: 14.0,
        y: 12.0,
        w: 7.0,
        h: 1.0,
        color: Pal::Yellow,
        out_at: 11.9,
    },
    ShipLight {
        x: 7.0,
        y: 12.0,
        w: 7.0,
        h: 1.0,
        color: Pal::Yellow,
        out_at: 12.4,
    },
    ShipLight {
        x: 13.0,
        y: 2.0,
        w: 1.0,
        h: 1.0,
        color: Pal::Red,
        out_at: ALL_DARK_AT,
    },
];

/// Seconds a light flickers before going out for good.
const FLICKER: f32 = 1.2;

/// Whether a light is showing at time `t`. It stutters, then stays off.
pub fn light_is_on(out_at: f32, t: f32) -> bool {
    if t >= out_at {
        return false;
    }
    if t < out_at - FLICKER {
        return true;
    }
    // The gaps lengthen as the light fails.
    let left = (out_at - t) / FLICKER;
    (t * 11.0).sin() + left * 1.4 > 0.0
}

/// Height of the sea above the waterline, at a column and a time.
pub fn sea_height(x: f32, t: f32) -> f32 {
    let chop = ((x * 0.035 - t * 1.7).sin() + (x * 0.09 + t * 1.1).sin() * 0.5) * SEA_AMPLITUDE;
    let swells: f32 = SEAS_AT
        .iter()
        .zip(SWELL_PEAK)
        .map(|(start, peak)| {
            let center = -SWELL_WIDTH * 2.0 + (t - start) * SWELL_SPEED;
            let distance = (x - center) / SWELL_WIDTH;
            peak * (-distance * distance).exp()
        })
        .sum();
    chop + swells + cover_height(t)
}

/// The rise of the whole sea over the view, and its fall back.
pub fn cover_height(t: f32) -> f32 {
    let distance = (t - COVERED_AT).abs() / COVER_RISE;
    if distance >= 1.0 {
        return 0.0;
    }
    let shape = (distance * std::f32::consts::FRAC_PI_2).cos();
    shape * shape * COVER_HEIGHT
}

/// The ship is shown until the sea stands fully over her.
pub fn ship_visible(t: f32) -> bool {
    t < COVERED_AT
}

/// Feet below the surface during the descent.
pub fn depth_at(t: f32) -> f32 {
    ((t - DESCENT_AT) / (WRECK_AT - DESCENT_AT)).clamp(0.0, 1.0) * WRECK_DEPTH_FT
}

/// Wind volume, from full in the storm to silence over the empty sea.
pub fn wind_volume(t: f32) -> f32 {
    (1.0 - (t - WIND_FADE_FROM) / (BLACK_AT - WIND_FADE_FROM)).clamp(0.0, 1.0) * WIND_VOLUME
}

/// Water color at a depth: light leaves as the view goes down.
fn water_color(depth: f32) -> Pal {
    if depth < WRECK_DEPTH_FT * 0.3 {
        Pal::Deep
    } else if depth < WRECK_DEPTH_FT * 0.65 {
        Pal::Night
    } else {
        Pal::Black
    }
}

/// Everything in the sequence, removed when the words come up.
#[derive(Component, Clone)]
pub struct ClosingPart;

#[derive(Component, Clone)]
struct SurfacePart;

#[derive(Component, Clone)]
struct DepthPart;

#[derive(Component)]
struct ShipBody;

#[derive(Component)]
struct LightPart(usize);

#[derive(Component)]
struct SeaColumn {
    x: f32,
    crest: bool,
}

#[derive(Component)]
struct SnowFlake {
    x: f32,
    y: f32,
    fall: f32,
}

#[derive(Component)]
struct GoneText;

#[derive(Component)]
struct DepthText;

#[derive(Component)]
struct DepthWater;

#[derive(Component)]
struct FloorPart {
    y: f32,
}

#[derive(Component)]
struct Mote {
    x: f32,
    y: f32,
}

#[derive(Component)]
struct ClosingWind;

#[derive(Resource, Default)]
pub struct ClosingCues {
    last_t: f32,
}

/// A 1x1 sprite, scaled each frame. Size changes through `Transform` keep
/// Bevy's sprite material buckets stable.
fn unit_sprite(color: Pal, z_order: f32) -> (Sprite, Transform) {
    (
        Sprite {
            color: color.color(),
            custom_size: Some(Vec2::ONE),
            ..default()
        },
        at(0.0, 0.0, z_order),
    )
}

pub fn spawn_closing(
    commands: &mut Commands,
    sprites: &Sprites,
    content: &Content,
    ending: &Ending,
    rng: &mut GameRng,
    server: &AssetServer,
) {
    commands.insert_resource(ClosingCues::default());
    let scope = (DespawnOnExit(GameState::Outcome), ClosingPart);
    let surface = (scope.clone(), SurfacePart);

    commands.spawn((
        rect(0.0, 0.0, 320.0, 200.0, Pal::Night, z::BACKGROUND),
        surface.clone(),
    ));
    commands.spawn((
        Sprite::from_image(sprites.side.clone()),
        at(SHIP_X, SHIP_Y, z::SHIP),
        ShipBody,
        surface.clone(),
    ));
    for (index, light) in LIGHTS.iter().enumerate() {
        commands.spawn((
            rect(
                SHIP_LEFT + light.x,
                SHIP_TOP + light.y,
                light.w,
                light.h,
                light.color,
                z::SHIP + 1.0,
            ),
            LightPart(index),
            surface.clone(),
        ));
    }
    for column in 0..COLUMNS {
        let x = column as f32 * COLUMN_W;
        commands.spawn((
            unit_sprite(Pal::Deep, z::WEATHER),
            SeaColumn { x, crest: false },
            surface.clone(),
        ));
        commands.spawn((
            unit_sprite(Pal::Foam, z::WEATHER + 1.0),
            SeaColumn { x, crest: true },
            surface.clone(),
        ));
    }
    for _ in 0..FLAKES {
        commands.spawn((
            rect(0.0, 0.0, 1.0, 1.0, Pal::White, z::WEATHER + 2.0),
            SnowFlake {
                x: rng.range(0.0, 320.0),
                y: rng.range(0.0, 200.0),
                fall: rng.range(60.0, 130.0),
            },
            surface.clone(),
        ));
    }
    commands.spawn((
        centered(clock_text(ending.minute), 10.0, Pal::Gray, 1.0),
        surface.clone(),
    ));
    commands.spawn((
        centered(content.text.closing_gone.clone(), 150.0, Pal::Foam, 1.0),
        Visibility::Hidden,
        GoneText,
        surface,
    ));

    let depth = (scope.clone(), DepthPart);
    commands.spawn((
        rect(0.0, 0.0, 320.0, 200.0, Pal::Deep, z::BACKGROUND),
        DepthWater,
        Visibility::Hidden,
        depth.clone(),
    ));
    for _ in 0..24 {
        commands.spawn((
            rect(0.0, 0.0, 1.0, 1.0, Pal::Slate, z::FIELD),
            Mote {
                x: rng.range(0.0, 320.0),
                y: rng.range(0.0, 200.0),
            },
            Visibility::Hidden,
            depth.clone(),
        ));
    }
    // Two sections on the bottom, cut from the same drawing of the ship.
    for (sheet_x, screen_x, sink) in [(0.0, 86.0, 6.0), (SHIP_W - SECTION_W, 226.0, 4.0)] {
        let y = FLOOR_Y - SHIP_H / 2.0 + sink;
        commands.spawn((
            Sprite {
                image: sprites.silhouette.clone(),
                rect: Some(Rect::new(sheet_x, 0.0, sheet_x + SECTION_W, SHIP_H)),
                color: Pal::Slate.color(),
                ..default()
            },
            at(screen_x, y, z::SHIP),
            FloorPart { y },
            Visibility::Hidden,
            depth.clone(),
        ));
    }
    let floor_y = FLOOR_Y + 20.0;
    commands.spawn((
        rect(0.0, FLOOR_Y, 320.0, 40.0, Pal::Shore, z::SHIP + 1.0),
        FloorPart { y: floor_y },
        Visibility::Hidden,
        depth.clone(),
    ));
    commands.spawn((
        centered("", 20.0, Pal::Foam, 2.0),
        DepthText,
        Visibility::Hidden,
        depth.clone(),
    ));
    commands.spawn((
        centered(content.text.closing_position.clone(), 180.0, Pal::Gray, 1.0),
        FloorPart { y: 184.0 },
        Visibility::Hidden,
        depth,
    ));

    commands.spawn((
        AudioPlayer::new(server.load("audio/music_wind.ogg")),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(WIND_VOLUME)),
        ClosingWind,
        scope,
    ));
}

/// Run the sequence at time `t`, in seconds since it began.
pub fn draw_closing(
    t: f32,
    time: &Time,
    cues: &mut ClosingCues,
    sprites: &Sprites,
    sfx: &mut MessageWriter<Sfx>,
    world: &mut ClosingWorld,
) {
    let crossed = |mark: f32| cues.last_t < mark && t >= mark;
    for mark in SEAS_AT {
        if crossed(mark + 1.5) {
            sfx.write(Sfx::Rogue);
        }
    }
    if crossed(COVERED_AT - 1.0) {
        sfx.write(Sfx::Rogue);
    }
    if crossed(WRECK_AT) {
        sfx.write(Sfx::Bell);
    }
    cues.last_t = t;

    let show_surface = t < BLACK_AT;
    let show_depth = t >= DESCENT_AT;
    let shown = |on: bool| {
        if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        }
    };
    for mut visibility in &mut world.surface {
        *visibility = shown(show_surface);
    }

    // The ship goes to a black shape once her lights are out.
    for (mut sprite, mut visibility, mut transform) in &mut world.body {
        *visibility = shown(show_surface && ship_visible(t));
        if t >= ALL_DARK_AT {
            sprite.image = sprites.silhouette.clone();
            sprite.color = Pal::Black.color();
        }
        let bob = (t * 1.3).sin().round();
        transform.translation.y = 100.0 - (SHIP_Y + bob);
    }
    for (light, mut visibility) in &mut world.lights {
        let on = light_is_on(LIGHTS[light.0].out_at, t) && ship_visible(t);
        *visibility = shown(show_surface && on);
    }
    for (column, mut transform) in &mut world.columns {
        let height = sea_height(column.x + COLUMN_W / 2.0, t).round();
        let top = WATERLINE - height;
        let (y, h) = if column.crest {
            (top, 1.0)
        } else {
            (top, 200.0 - top + 1.0)
        };
        transform.translation.x = column.x + COLUMN_W / 2.0 - 160.0;
        transform.translation.y = 100.0 - y - h / 2.0;
        transform.scale = Vec3::new(COLUMN_W, h, 1.0);
    }
    for (mut flake, mut transform) in &mut world.flakes {
        let fall = flake.fall;
        flake.y = (flake.y + fall * time.delta_secs()) % 200.0;
        flake.x = (flake.x - fall * 0.7 * time.delta_secs()).rem_euclid(320.0);
        transform.translation.x = flake.x.round() - 160.0;
        transform.translation.y = 100.0 - flake.y.round();
    }
    for mut visibility in &mut world.gone {
        *visibility = shown(show_surface && t >= GONE_TEXT_AT);
    }
    for mut sink in &mut world.wind {
        sink.set_volume(Volume::Linear(wind_volume(t)));
    }

    // The way down.
    let depth = depth_at(t);
    for mut visibility in &mut world.depth {
        *visibility = shown(show_depth);
    }
    for mut sprite in &mut world.water {
        sprite.color = water_color(depth).color();
    }
    for (mut mote, mut transform) in &mut world.motes {
        if t < WRECK_AT {
            mote.y -= 70.0 * time.delta_secs();
            if mote.y < 0.0 {
                mote.y += 200.0;
            }
        }
        transform.translation.x = mote.x.round() - 160.0;
        transform.translation.y = 100.0 - mote.y.round();
    }
    let arrival = depth / WRECK_DEPTH_FT;
    let rise = ((arrival - FLOOR_RISE_FROM) / (1.0 - FLOOR_RISE_FROM)).clamp(0.0, 1.0);
    let below = ((1.0 - rise) * 110.0).round();
    for (floor, mut transform, mut visibility) in &mut world.floor {
        transform.translation.y = 100.0 - (floor.y + below);
        *visibility = shown(show_depth && rise > 0.0);
    }
    for mut text in &mut world.depth_text {
        text.set(format!("{:>3.0} FEET", depth));
    }
}

/// The queries the sequence draws through.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ClosingWorld<'w, 's> {
    surface: Query<
        'w,
        's,
        &'static mut Visibility,
        (
            With<SurfacePart>,
            Without<ShipBody>,
            Without<LightPart>,
            Without<GoneText>,
        ),
    >,
    body: Query<
        'w,
        's,
        (
            &'static mut Sprite,
            &'static mut Visibility,
            &'static mut Transform,
        ),
        With<ShipBody>,
    >,
    lights: Query<
        'w,
        's,
        (&'static LightPart, &'static mut Visibility),
        (Without<ShipBody>, Without<FloorPart>),
    >,
    columns: Query<
        'w,
        's,
        (&'static SeaColumn, &'static mut Transform),
        (
            Without<ShipBody>,
            Without<SnowFlake>,
            Without<Mote>,
            Without<FloorPart>,
        ),
    >,
    flakes: Query<
        'w,
        's,
        (&'static mut SnowFlake, &'static mut Transform),
        (Without<ShipBody>, Without<Mote>, Without<FloorPart>),
    >,
    gone: Query<
        'w,
        's,
        &'static mut Visibility,
        (
            With<GoneText>,
            Without<FloorPart>,
            Without<DepthPart>,
            Without<ShipBody>,
            Without<LightPart>,
        ),
    >,
    wind: Query<'w, 's, &'static mut AudioSink, With<ClosingWind>>,
    depth: Query<
        'w,
        's,
        &'static mut Visibility,
        (
            With<DepthPart>,
            Without<SurfacePart>,
            Without<FloorPart>,
            Without<ShipBody>,
            Without<LightPart>,
        ),
    >,
    water: Query<'w, 's, &'static mut Sprite, (With<DepthWater>, Without<ShipBody>)>,
    motes: Query<
        'w,
        's,
        (&'static mut Mote, &'static mut Transform),
        (Without<ShipBody>, Without<FloorPart>),
    >,
    floor: Query<
        'w,
        's,
        (
            &'static FloorPart,
            &'static mut Transform,
            &'static mut Visibility,
        ),
        (Without<ShipBody>, Without<SurfacePart>),
    >,
    depth_text: Query<'w, 's, &'static mut Label, With<DepthText>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beats_run_in_order() {
        let mut last = 0.0;
        for mark in [
            SEAS_AT[0],
            SEAS_AT[1],
            SEAS_AT[2],
            ALL_DARK_AT,
            COVERED_AT,
            GONE_TEXT_AT,
            BLACK_AT,
            DESCENT_AT,
            WRECK_AT,
            CLOSING_SECONDS,
        ] {
            assert!(mark > last, "{mark}");
            last = mark;
        }
    }

    #[test]
    fn lights_are_steady_then_stutter_then_stay_out() {
        assert!(light_is_on(10.0, 0.0));
        assert!(light_is_on(10.0, 8.7));
        let stutter: Vec<bool> = (0..120)
            .map(|step| light_is_on(10.0, 8.8 + step as f32 * 0.01))
            .collect();
        assert!(stutter.contains(&false) && stutter.contains(&true));
        for later in [10.0, 10.5, 40.0] {
            assert!(!light_is_on(10.0, later));
        }
    }

    #[test]
    fn every_light_is_out_by_the_time_she_goes_dark() {
        for light in &LIGHTS {
            assert!(light.out_at <= ALL_DARK_AT);
            assert!(!light_is_on(light.out_at, ALL_DARK_AT));
        }
    }

    #[test]
    fn the_sea_stands_over_the_whole_ship_before_she_is_removed() {
        let top_of_ship = WATERLINE - (SHIP_TOP - 1.0);
        for offset in 0..=(SHIP_W as i32) {
            let x = SHIP_LEFT + offset as f32;
            assert!(sea_height(x, COVERED_AT) > top_of_ship, "column {x}");
        }
    }

    #[test]
    fn the_sea_falls_back_and_she_is_not_there() {
        assert_eq!(cover_height(COVERED_AT + COVER_RISE), 0.0);
        assert!(!ship_visible(COVERED_AT + COVER_RISE));
        assert!(ship_visible(COVERED_AT - 0.1));
    }

    #[test]
    fn each_of_the_three_seas_is_larger_than_the_last() {
        const { assert!(SWELL_PEAK[0] < SWELL_PEAK[1] && SWELL_PEAK[1] < SWELL_PEAK[2]) };
    }

    #[test]
    fn the_descent_ends_at_the_charted_depth() {
        assert_eq!(depth_at(DESCENT_AT), 0.0);
        assert_eq!(depth_at(WRECK_AT), 530.0);
        assert_eq!(depth_at(CLOSING_SECONDS), 530.0);
    }

    #[test]
    fn the_wind_dies_before_the_cut_to_black() {
        assert_eq!(wind_volume(0.0), WIND_VOLUME);
        assert_eq!(wind_volume(BLACK_AT), 0.0);
    }

    #[test]
    fn the_water_darkens_on_the_way_down() {
        assert_eq!(water_color(0.0), Pal::Deep);
        assert_eq!(water_color(WRECK_DEPTH_FT), Pal::Black);
    }
}
