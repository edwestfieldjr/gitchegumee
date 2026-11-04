//! The voyage screen: the chart, ship status, radio log, and orders.

use bevy::prelude::*;

use super::map::{MAP_H, MAP_W, MapImage, project};
use super::route::path_point;
use super::{ClockRate, Voyage};
use crate::content::{Content, Effect, SpeedOrder, clock_text};
use crate::events::ApplyEffect;
use crate::ship::{PUMPS, Pump, Ship};
use crate::states::GameState;
use crate::ui::idle::Busy;
use crate::ui::palette::Pal;
use crate::ui::{Bar, Label, advance_pressed, at, bar_parts, label, rect, z};
use crate::weather::{Weather, compass_point};

const MAP_Y: f32 = 10.0;
const SIDE_X: f32 = 228.0;
const LOG_Y: f32 = 133.0;
const LOG_ROWS: usize = 5;
const PATH_DOT_NM: f32 = 7.0;
/// Game seconds the clock holds so a new radio log line can be read. At a `pace`
/// of 0.4375 that is about 1.8 real seconds.
const LOG_READ_SECONDS: f32 = 0.8;
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

/// The radio log lines already seen, and how long the clock is still holding for
/// the newest of them.
#[derive(Resource, Default)]
struct LogHold {
    seen: usize,
    left: f32,
}

impl LogHold {
    /// Move on by `seconds` of game time, and say whether the clock should hold.
    /// Lines that arrived while waiting for orders can already be read at leisure.
    fn step(&mut self, log_lines: usize, proceeding: bool, seconds: f32) -> bool {
        if !proceeding {
            self.seen = log_lines;
            self.left = 0.0;
        } else if log_lines > self.seen {
            self.seen = log_lines;
            self.left = LOG_READ_SECONDS;
        } else {
            self.left = (self.left - seconds).max(0.0);
        }
        self.left > 0.0
    }
}

/// The voyage clock while the chart is up, in minutes per game second.
fn chart_rate(proceeding: bool, holding: bool, minutes_per_second: f32) -> f32 {
    if proceeding && !holding {
        minutes_per_second
    } else {
        0.0
    }
}

fn hint_text(proceeding: bool) -> &'static str {
    if proceeding {
        "UNDER WAY  KEEP CLEAR OF BREAKERS AND DEBRIS"
    } else {
        "UP/DN SPEED 1-6 PUMPS R REPAIR ENTER: GO"
    }
}

/// While the clock waits for orders the hint has to stand out, or the chart just
/// looks stuck.
fn hint_color(proceeding: bool) -> Pal {
    if proceeding { Pal::Gray } else { Pal::Yellow }
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
    rate.0 = chart_rate(
        proceeding.0,
        false,
        content.balance.voyage_minutes_per_second,
    );
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
        label(
            hint_text(proceeding.0),
            4.0,
            190.0,
            hint_color(proceeding.0),
        ),
        HintLabel,
        scope,
    ));
}

fn voyage_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    ship: Res<Ship>,
    voyage: Res<Voyage>,
    content: Res<Content>,
    mut proceeding: ResMut<Proceeding>,
    mut log_hold: ResMut<LogHold>,
    mut rate: ResMut<ClockRate>,
    mut hint: Single<&mut Label, With<HintLabel>>,
    mut effects: MessageWriter<ApplyEffect>,
    mut busy: MessageWriter<Busy>,
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
    // Under way, the chart is running and not waiting for anyone.
    if proceeding.0 {
        busy.write(Busy);
    }
    let holding = log_hold.step(voyage.logged, proceeding.0, time.delta_secs());
    rate.0 = chart_rate(
        proceeding.0,
        holding,
        content.balance.voyage_minutes_per_second,
    );
    hint.set(hint_text(proceeding.0));
    let color = hint_color(proceeding.0);
    if hint.color != color {
        hint.color = color;
    }
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
                "{:>3.0} NM TO WHITEFISH POINT",
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
            .init_resource::<LogHold>()
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
    fn the_clock_waits_for_orders_and_for_the_radio_log() {
        assert_eq!(chart_rate(false, false, 520.0), 0.0);
        assert_eq!(chart_rate(true, true, 520.0), 0.0);
        assert_eq!(chart_rate(true, false, 520.0), 520.0);
    }

    #[test]
    fn a_new_log_line_holds_the_clock_long_enough_to_read() {
        let mut hold = LogHold::default();
        assert!(!hold.step(0, true, 0.1));
        assert!(hold.step(1, true, 0.1));
        assert!(hold.step(1, true, LOG_READ_SECONDS / 2.0));
        assert!(!hold.step(1, true, LOG_READ_SECONDS));
    }

    #[test]
    fn each_new_log_line_gets_its_own_time() {
        let mut hold = LogHold::default();
        hold.step(1, true, 0.1);
        hold.step(1, true, LOG_READ_SECONDS - 0.01);
        assert!(hold.step(2, true, 0.1));
        assert_eq!(hold.left, LOG_READ_SECONDS);
    }

    #[test]
    fn lines_logged_while_waiting_for_orders_do_not_hold_the_clock_later() {
        let mut hold = LogHold::default();
        assert!(!hold.step(3, false, 0.1));
        assert!(!hold.step(3, true, 0.1));
    }

    #[test]
    fn a_pause_does_not_use_up_the_reading_time() {
        let mut hold = LogHold::default();
        hold.step(1, true, 0.0);
        let left = hold.left;
        hold.step(1, true, 0.0);
        assert_eq!(hold.left, left);
    }

    #[test]
    fn the_hints_stop_short_of_the_pause_and_mute_keys() {
        for proceeding in [false, true] {
            let right = 4.0 + hint_text(proceeding).len() as f32 * 6.0;
            assert!(right <= 270.0, "{right}");
        }
    }

    #[test]
    fn the_waiting_hint_stands_out() {
        assert_ne!(hint_color(false), hint_color(true));
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
