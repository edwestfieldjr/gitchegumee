//! The voyage screen: the chart, ship status, radio log, and orders.

use bevy::prelude::*;

use super::map::{MAP_H, MAP_W, MapImage, project};
use super::route::path_point;
use super::{ClockRate, Voyage};
use crate::content::{Content, Effect, SpeedOrder, clock_text};
use crate::events::ApplyEffect;
use crate::ship::{PUMPS, Pump, Ship};
use crate::states::GameState;
use crate::ui::palette::Pal;
use crate::ui::{Bar, Label, advance_pressed, at, bar_parts, label, rect, z};
use crate::weather::{Weather, compass_point};

const MAP_Y: f32 = 10.0;
const SIDE_X: f32 = 228.0;
const LOG_Y: f32 = 133.0;
const LOG_ROWS: usize = 6;
const PATH_DOT_NM: f32 = 7.0;
/// Miles the Anderson drops astern by the time she loses sight of the ship.
const ANDERSON_MAX_LAG_NM: f32 = 14.0;
const ANDERSON_KEPT_LAG_NM: f32 = 5.0;
/// Hours from the storm's arrival over which the lag opens.
const ANDERSON_LAG_HOURS: f32 = 12.0;
const ANDERSON_LAG_FROM: f32 = 1620.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Readout {
    Clock,
    Route,
    List,
    Freeboard,
    Speed,
    Crew,
    Wind,
    Seas,
    Radar,
    Lee,
}

#[derive(Component)]
struct ReadoutLabel(Readout);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gauge {
    Hull,
    Water,
    Fuel,
}

#[derive(Component)]
struct GaugeBar(Gauge);

#[derive(Component)]
struct PumpLabel(usize);

#[derive(Component)]
struct LogRow(usize);

/// Whether the captain has given orders. The chart's clock waits until then.
#[derive(Resource, Default)]
struct Proceeding(bool);

fn stop_for_orders(mut proceeding: ResMut<Proceeding>) {
    proceeding.0 = false;
}

/// The voyage clock while the chart is up, in minutes per game second.
fn chart_rate(proceeding: bool, minutes_per_second: f32) -> f32 {
    if proceeding { minutes_per_second } else { 0.0 }
}

fn hint_text(proceeding: bool) -> &'static str {
    if proceeding {
        "UNDER WAY"
    } else {
        "UP/DN SPEED  1-6 PUMPS  R REPAIR  SPACE/ENTER: GO"
    }
}

#[derive(Component)]
struct HintLabel;

#[derive(Component)]
struct ShipMarker;

#[derive(Component)]
struct AndersonMarker;

fn map_transform(position: Vec2, z_order: f32) -> Transform {
    at(position.x.floor(), MAP_Y + position.y.floor(), z_order)
}

fn enter_voyage(
    mut commands: Commands,
    map: Res<MapImage>,
    voyage: Res<Voyage>,
    proceeding: Res<Proceeding>,
    mut rate: ResMut<ClockRate>,
    content: Res<Content>,
) {
    rate.0 = chart_rate(proceeding.0, content.balance.voyage_minutes_per_second);
    let scope = DespawnOnExit(GameState::Voyage);
    commands.spawn((
        rect(0.0, 0.0, 320.0, 200.0, Pal::Night, z::BACKGROUND),
        scope.clone(),
    ));
    commands.spawn((
        Sprite::from_image(map.0.clone()),
        at(MAP_W as f32 / 2.0, MAP_Y + MAP_H as f32 / 2.0, z::FIELD),
        scope.clone(),
    ));

    let length = voyage.length_nm();
    let dots = (length / PATH_DOT_NM) as usize;
    for dot in 0..=dots {
        let spot = project(path_point(&voyage.path, dot as f32 * PATH_DOT_NM));
        commands.spawn((
            Sprite {
                color: Pal::Sky.color(),
                custom_size: Some(Vec2::ONE),
                ..default()
            },
            map_transform(spot, z::FIELD + 1.0),
            scope.clone(),
        ));
    }
    commands.spawn((
        Sprite {
            color: Pal::White.color(),
            custom_size: Some(Vec2::splat(3.0)),
            ..default()
        },
        map_transform(Vec2::ZERO, z::SHIP),
        AndersonMarker,
        scope.clone(),
    ));
    commands.spawn((
        Sprite {
            color: Pal::Yellow.color(),
            custom_size: Some(Vec2::splat(3.0)),
            ..default()
        },
        map_transform(Vec2::ZERO, z::SHIP + 1.0),
        ShipMarker,
        scope.clone(),
    ));

    commands.spawn((
        label("", 2.0, 1.0, Pal::White),
        ReadoutLabel(Readout::Clock),
        scope.clone(),
    ));
    commands.spawn((
        label("", 112.0, 1.0, Pal::Sky),
        ReadoutLabel(Readout::Route),
        scope.clone(),
    ));

    for (text, y, gauge) in [
        ("HULL", 12.0, Gauge::Hull),
        ("WATER", 24.0, Gauge::Water),
        ("FUEL", 36.0, Gauge::Fuel),
    ] {
        commands.spawn((label(text, SIDE_X, y, Pal::Gray), scope.clone()));
        let fill = match gauge {
            Gauge::Hull => Pal::Pine,
            Gauge::Water => Pal::Red,
            Gauge::Fuel => Pal::Amber,
        };
        let (back, front) = bar_parts(
            Bar::new(SIDE_X + 36.0, y + 1.0, 52.0, 5.0),
            fill,
            Pal::Slate,
        );
        commands.spawn((back, scope.clone()));
        commands.spawn((front, GaugeBar(gauge), scope.clone()));
    }
    for (readout, y) in [
        (Readout::List, 48.0),
        (Readout::Freeboard, 57.0),
        (Readout::Speed, 66.0),
        (Readout::Wind, 75.0),
        (Readout::Seas, 84.0),
        (Readout::Crew, 93.0),
        (Readout::Radar, 102.0),
        (Readout::Lee, 120.0),
    ] {
        commands.spawn((
            label("", SIDE_X, y, Pal::White),
            ReadoutLabel(readout),
            scope.clone(),
        ));
    }
    commands.spawn((label("PUMP", SIDE_X, 111.0, Pal::Gray), scope.clone()));
    for index in 0..PUMPS {
        commands.spawn((
            label(
                (index + 1).to_string(),
                SIDE_X + 30.0 + index as f32 * 9.0,
                111.0,
                Pal::Gray,
            ),
            PumpLabel(index),
            scope.clone(),
        ));
    }

    commands.spawn((
        rect(0.0, 130.0, 320.0, 1.0, Pal::Slate, z::HUD),
        scope.clone(),
    ));
    for row in 0..LOG_ROWS {
        commands.spawn((
            label("", 4.0, LOG_Y + row as f32 * 8.0, Pal::Foam),
            LogRow(row),
            scope.clone(),
        ));
    }
    commands.spawn((
        label(hint_text(proceeding.0), 4.0, 190.0, Pal::Gray),
        HintLabel,
        scope,
    ));
}

fn voyage_input(
    keys: Res<ButtonInput<KeyCode>>,
    ship: Res<Ship>,
    content: Res<Content>,
    mut proceeding: ResMut<Proceeding>,
    mut rate: ResMut<ClockRate>,
    mut hint: Single<&mut Label, With<HintLabel>>,
    mut effects: MessageWriter<ApplyEffect>,
) {
    const PUMP_KEYS: [KeyCode; PUMPS] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
    ];
    for (index, key) in PUMP_KEYS.into_iter().enumerate() {
        if keys.just_pressed(key) {
            effects.write(ApplyEffect(Effect::TogglePump(index)));
        }
    }
    if keys.just_pressed(KeyCode::KeyR) {
        effects.write(ApplyEffect(Effect::RepairPump));
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        let faster = match ship.order {
            SpeedOrder::Slow => SpeedOrder::Cruise,
            SpeedOrder::Cruise | SpeedOrder::Full => SpeedOrder::Full,
        };
        effects.write(ApplyEffect(Effect::Speed(faster)));
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        let slower = match ship.order {
            SpeedOrder::Full => SpeedOrder::Cruise,
            SpeedOrder::Cruise | SpeedOrder::Slow => SpeedOrder::Slow,
        };
        effects.write(ApplyEffect(Effect::Speed(slower)));
    }
    if advance_pressed(&keys) {
        proceeding.0 = true;
    }
    rate.0 = chart_rate(proceeding.0, content.balance.voyage_minutes_per_second);
    hint.set(hint_text(proceeding.0));
}

/// Miles the Anderson is astern at a clock minute.
fn anderson_lag(minute: f32, kept_pace: bool) -> f32 {
    let limit = if kept_pace {
        ANDERSON_KEPT_LAG_NM
    } else {
        ANDERSON_MAX_LAG_NM
    };
    ((minute - ANDERSON_LAG_FROM) / 60.0 / ANDERSON_LAG_HOURS).clamp(0.0, 1.0) * limit
}

fn order_name(order: SpeedOrder) -> &'static str {
    match order {
        SpeedOrder::Slow => "SLOW",
        SpeedOrder::Cruise => "CRUISE",
        SpeedOrder::Full => "FULL",
    }
}

fn draw_status(
    ship: Res<Ship>,
    voyage: Res<Voyage>,
    weather: Res<Weather>,
    content: Res<Content>,
    mut readouts: Query<(&ReadoutLabel, &mut Label), (Without<PumpLabel>, Without<LogRow>)>,
    mut gauges: Query<(&GaugeBar, &mut Bar)>,
    mut pumps: Query<(&PumpLabel, &mut Label), (Without<ReadoutLabel>, Without<LogRow>)>,
    mut log: Query<(&LogRow, &mut Label), (Without<ReadoutLabel>, Without<PumpLabel>)>,
) {
    let tuning = &content.balance.ship;
    for (readout, mut text) in &mut readouts {
        let side = if ship.list_deg >= 0.0 { "STBD" } else { "PORT" };
        text.set(match readout.0 {
            Readout::Clock => clock_text(voyage.minute),
            Readout::Route => format!(
                "{:>3.0} NM TO WHITEFISH BAY",
                voyage.length_nm() - voyage.nm
            ),
            Readout::List => format!("LIST  {:>2.0} {side}", ship.list_deg.abs()),
            Readout::Freeboard => format!("DECK  {:>4.1} FT", ship.freeboard_ft(tuning)),
            Readout::Speed => format!("{:<6} {:>2.0} KN", order_name(ship.order), ship.speed_kn),
            Readout::Wind => format!(
                "WIND {:>2.0} KN {}",
                weather.wind_kn,
                compass_point(weather.wind_from)
            ),
            Readout::Seas => format!("SEAS {:>2.0} FT", weather.wave_ft),
            Readout::Crew => format!("TIRED {:>2.0} MRL {:>2.0}", ship.fatigue, ship.morale),
            Readout::Radar => {
                let state = |up: bool| if up { "UP" } else { "--" };
                format!("RADAR {} {}", state(ship.radars[0]), state(ship.radars[1]))
            }
            Readout::Lee => {
                if voyage.in_lee() {
                    "IN THE LEE".to_string()
                } else {
                    String::new()
                }
            }
        });
    }
    for (gauge, mut bar) in &mut gauges {
        let value = match gauge.0 {
            Gauge::Hull => ship.hull,
            Gauge::Water => ship.water,
            Gauge::Fuel => ship.fuel,
        } / 100.0;
        bar.value = value;
    }
    for (pump, mut text) in &mut pumps {
        let color = match ship.pumps[pump.0] {
            Pump::Standby => Pal::Gray,
            Pump::Running => Pal::Pine,
            Pump::Failed => Pal::Red,
        };
        if text.color != color {
            text.color = color;
        }
    }
    let start = voyage.log.len().saturating_sub(LOG_ROWS);
    for (row, mut text) in &mut log {
        text.set(voyage.log.get(start + row.0).cloned().unwrap_or_default());
    }
}

fn draw_markers(
    ship: Res<Ship>,
    voyage: Res<Voyage>,
    mut own: Query<&mut Transform, (With<ShipMarker>, Without<AndersonMarker>)>,
    mut other: Query<(&mut Transform, &mut Visibility), With<AndersonMarker>>,
) {
    let place = |spot: Vec2, z_order: f32| map_transform(spot - Vec2::splat(1.0), z_order);
    for mut transform in &mut own {
        transform.translation = place(project(voyage.position()), z::SHIP + 1.0).translation;
    }
    let lag = anderson_lag(voyage.minute, voyage.has_flag("kept_pace"));
    let spot = project(path_point(&voyage.path, (voyage.nm - lag).max(0.0)));
    for (mut transform, mut visibility) in &mut other {
        transform.translation = place(spot, z::SHIP).translation;
        // Sailing in company until 5 p.m.; only radar shows her after that.
        let in_sight = voyage.minute >= 1020.0 && ship.radar_up();
        *visibility = if in_sight {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

pub struct VoyageScreenPlugin;

impl Plugin for VoyageScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Proceeding>()
            .add_systems(OnEnter(GameState::Port), stop_for_orders)
            .add_systems(OnExit(GameState::Helm), stop_for_orders)
            .add_systems(OnEnter(GameState::Voyage), enter_voyage)
            .add_systems(
                Update,
                (voyage_input, draw_status, draw_markers).run_if(in_state(GameState::Voyage)),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::At;
    use crate::content::tests::real_content;

    #[test]
    fn the_clock_waits_for_orders() {
        assert_eq!(chart_rate(false, 520.0), 0.0);
        assert_eq!(chart_rate(true, 520.0), 520.0);
    }

    #[test]
    fn no_leg_keeps_the_player_on_the_chart_for_long() {
        let content = real_content();
        let mut from = At(9, 14, 15).minute();
        let real_rate = content.balance.voyage_minutes_per_second * content.balance.pace;
        for section in &content.helm {
            let seconds = (section.at.minute() - from) / real_rate;
            assert!(seconds <= 6.0, "{} is {seconds} seconds away", section.id);
            from = section.at.minute() + section.minutes;
        }
    }

    #[test]
    fn the_anderson_opens_to_her_historical_distance() {
        assert_eq!(anderson_lag(0.0, false), 0.0);
        assert_eq!(anderson_lag(99_999.0, false), ANDERSON_MAX_LAG_NM);
    }

    #[test]
    fn keeping_pace_holds_her_closer() {
        assert_eq!(anderson_lag(99_999.0, true), ANDERSON_KEPT_LAG_NM);
    }
}
